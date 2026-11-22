//! Owner-only, bounded offline collection reading copies. No remote image fetching.
use crate::{
    auth::context::AuthUser, db::CollectionExport, recipe_html::render, storage::StorageClient,
};
use axum::{
    extract::{Extension, Path, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    fmt::Write as _,
    io::{Cursor, Write as _},
    time::Duration,
};
use uuid::Uuid;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const ZIP_RESERVE: usize = 64 * 1024;

#[derive(Clone, Copy, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Layout {
    #[default]
    Unified,
    Individual,
}
#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Options {
    #[serde(default)]
    layout: Layout,
    #[serde(default)]
    images: bool,
}

pub(crate) async fn download(
    Path(id): Path<Uuid>,
    Query(options): Query<Options>,
    Extension(pool): Extension<sqlx::PgPool>,
    Extension(storage): Extension<StorageClient>,
    viewer: Option<Extension<AuthUser>>,
) -> Response {
    let result = match viewer {
        None => Err((
            StatusCode::UNAUTHORIZED,
            "Sign in to export your collection.".into(),
        )),
        Some(Extension(actor)) => {
            if crate::security::enforce_user_request_budget(
                actor.user_id,
                crate::security::RequestClass::CollectionRead,
            )
            .is_err()
            {
                Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    "Too many exports. Try again shortly.".into(),
                ))
            } else {
                match crate::security::run_with_expensive_read_work_budget(tokio::time::timeout(
                    Duration::from_secs(30),
                    prepare(
                        &pool,
                        &storage,
                        actor.user_id,
                        id,
                        options.layout,
                        options.images,
                    ),
                ))
                .await
                {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err((
                        StatusCode::GATEWAY_TIMEOUT,
                        "Export timed out. Try a smaller collection or text-only HTML.".into(),
                    )),
                    Err(_) => Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        "Exports are busy. Try again shortly.".into(),
                    )),
                }
            }
        }
    };
    let mut response = match result {
        Ok(bytes) => {
            let (mime, extension) = if options.layout == Layout::Unified {
                ("text/html; charset=utf-8", "html")
            } else {
                ("application/zip", "zip")
            };
            (
                [
                    (header::CONTENT_TYPE, mime.to_owned()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"noms-collection-{id}.{extension}\""),
                    ),
                ],
                bytes,
            )
                .into_response()
        }
        Err(error) => error.into_response(),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response
}

async fn prepare(
    pool: &sqlx::PgPool,
    storage: &StorageClient,
    owner: Uuid,
    id: Uuid,
    layout: Layout,
    images: bool,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let data = crate::security::run_with_database_budget(
        crate::security::DATABASE_EXECUTION_BUDGET,
        crate::db::read_collection_export(pool, owner, id),
    )
    .await
    .map_err(|_| {
        (
            StatusCode::GATEWAY_TIMEOUT,
            "Collection read timed out.".into(),
        )
    })?
    .map_err(|error| (StatusCode::BAD_REQUEST, error))?;
    build(storage, &data, layout, images, MAX_BYTES)
        .await
        .map_err(|error| (StatusCode::PAYLOAD_TOO_LARGE, error))
}

fn contents(data: &CollectionExport, layout: Layout) -> String {
    fn node(
        out: &mut String,
        data: &CollectionExport,
        index: usize,
        links: &HashMap<String, (usize, &str)>,
        layout: Layout,
    ) {
        let collection = &data.collections[index];
        write!(
            out,
            "<li><strong>{}</strong>",
            render::escape(&collection.name)
        )
        .unwrap();
        if let Some(description) = &collection.description {
            write!(
                out,
                "<p class=\"authored collection-description\">{}</p>",
                render::escape(description)
            )
            .unwrap();
        }
        if collection.recipes.is_empty() {
            out.push_str(
                "<p class=\"empty-collection\">No recipes directly in this collection.</p>",
            );
        }
        if !collection.recipes.is_empty() {
            out.push_str("<ol class=\"contents-recipes\">");
            for recipe in &collection.recipes {
                let (number, title) = links[recipe];
                let href = if layout == Layout::Unified {
                    format!("#recipe-{number}")
                } else {
                    format!("recipe-{number:03}.html")
                };
                write!(
                    out,
                    "<li><a href=\"{href}\">{}</a></li>",
                    render::escape(title)
                )
                .unwrap();
            }
            out.push_str("</ol>");
        }
        let children = data
            .collections
            .iter()
            .enumerate()
            .filter(|(_, c)| c.parent_collection.as_ref() == Some(&collection.portable_id))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if !children.is_empty() {
            out.push_str("<ul class=\"collection-tree\">");
            for child in children {
                node(out, data, child, links, layout);
            }
            out.push_str("</ul>");
        }
        out.push_str("</li>");
    }
    let mut out = render::start_document(&data.collections[0].name);
    write!(out, "<header class=\"card hero\"><span class=\"eyebrow\">Your offline cookbook</span><h1>{}</h1><p>{} {} · {} unique {}</p><p>A reading copy for offline viewing, sharing and printing. Use JSON to re-import into Noms. Collection covers, drafts and history are not included.</p>",
        render::escape(&data.collections[0].name), data.collections.len(), if data.collections.len() == 1 { "collection" } else { "collections" }, data.recipes.len(), if data.recipes.len() == 1 { "recipe" } else { "recipes" }).unwrap();
    if layout == Layout::Individual {
        out.push_str("<p>Open a recipe below. Each recipe HTML file also works on its own; keep this index with the recipe files to use these links.</p>");
    }
    out.push_str("</header><nav id=\"contents\" class=\"card contents\" aria-label=\"Cookbook contents\"><h2>Contents</h2><ul class=\"collection-tree\">");
    let links = data
        .recipes
        .iter()
        .enumerate()
        .map(|(i, (recipe, _))| {
            (
                format!("urn:noms:recipe:{}", recipe.id),
                (i + 1, recipe.title.as_str()),
            )
        })
        .collect();
    node(&mut out, data, 0, &links, layout);
    out.push_str("</ul></nav>");
    out
}

async fn build(
    storage: &StorageClient,
    data: &CollectionExport,
    layout: Layout,
    images: bool,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let placements = data
        .recipes
        .iter()
        .map(|(recipe, _)| super::recipe_html::image_references(recipe).len())
        .sum::<usize>();
    if placements > crate::collection_json::MAX_IMAGES {
        return Err("Export a smaller collection (up to 128 recipe image placements).".into());
    }
    let mut index = contents(data, layout);
    let footer = render::footer(images);
    let mut total = index.len() + footer.len() + ZIP_RESERVE;
    if total > limit {
        return Err("HTML exceeds 32 MiB. Export a smaller collection.".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (i, (recipe, tags)) in data.recipes.iter().enumerate() {
        let rendered = super::recipe_html::prepare_saved(
            storage,
            recipe,
            tags,
            images,
            deadline,
            limit.saturating_sub(total),
            (layout == Layout::Unified).then_some(i + 1),
        )
        .await?;
        total += rendered.html.len();
        if layout == Layout::Unified {
            index.push_str(&rendered.html);
        } else {
            write_zip(
                &mut zip,
                &format!("recipe-{:03}.html", i + 1),
                rendered.html.as_bytes(),
            )?;
        }
    }
    index.push_str(&footer);
    if layout == Layout::Unified {
        return Ok(index.into_bytes());
    }
    write_zip(&mut zip, "index.html", index.as_bytes())?;
    let bytes = zip
        .finish()
        .map_err(|_| "Could not finish the HTML ZIP.")?
        .into_inner();
    if bytes.len() > limit {
        return Err("HTML ZIP exceeds 32 MiB. Export a smaller collection.".into());
    }
    Ok(bytes)
}

fn write_zip(
    zip: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    name: &str,
    bytes: &[u8],
) -> Result<(), String> {
    // Generated numeric filenames keep duplicate or hostile recipe titles out of paths.
    zip.start_file(
        name,
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )
    .map_err(|_| "Could not prepare the HTML ZIP.")?;
    zip.write_all(bytes)
        .map_err(|_| "Could not prepare the HTML ZIP.".into())
}

#[cfg(test)]
#[path = "collection_html_tests.rs"]
mod tests;
