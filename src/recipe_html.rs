//! Standalone, inert reading copy. Canonical JSON remains the re-import format.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HtmlExport {
    pub html: String,
    pub warnings: Vec<String>,
}

#[cfg(feature = "server")]
pub(crate) mod render {
    use crate::types::{Recipe, RecipeImage, RecipeStep};
    use std::{collections::HashMap, fmt::Write};

    pub fn escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
    fn paragraph(out: &mut String, text: Option<&str>, class: &str) {
        if let Some(text) = text.filter(|s| !s.is_empty()) {
            write!(out, "<p class=\"{class}\">{}</p>", escape(text)).unwrap();
        }
    }
    fn photos(
        out: &mut String,
        images: &[RecipeImage],
        embedded: &HashMap<String, String>,
        label: &str,
    ) {
        if !images.iter().any(|image| embedded.contains_key(&image.url)) {
            return;
        }
        out.push_str("<div class=\"gallery\">");
        for (i, image) in images.iter().enumerate() {
            if let Some(uri) = embedded.get(&image.url) {
                // Only server-generated WebP data URIs may enter a resource attribute.
                if !uri.starts_with("data:image/webp;base64,") {
                    continue;
                }
                write!(out, "<figure class=\"photo\"><img src=\"{}\" alt=\"{} image {}\"><figcaption>{} · Image {}</figcaption></figure>", escape(uri), escape(label), i + 1, escape(label), i + 1).unwrap();
            }
        }
        out.push_str("</div>");
    }
    fn steps(
        out: &mut String,
        items: &[RecipeStep],
        embedded: &HashMap<String, String>,
        path: &str,
    ) {
        if items.is_empty() {
            return;
        }
        out.push_str("<ol class=\"steps\">");
        for (i, step) in items.iter().enumerate() {
            let label = format!("{path}{}", i + 1);
            out.push_str("<li>");
            paragraph(out, Some(&step.text), "authored");
            photos(out, &step.photos, embedded, &format!("Step {label}"));
            steps(out, &step.sub_steps, embedded, &format!("{label}."));
            out.push_str("</li>");
        }
        out.push_str("</ol>");
    }
    pub fn start_document(title: &str) -> String {
        let mut out = String::with_capacity(16 * 1024);
        write!(out, "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"referrer\" content=\"no-referrer\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'\"><title>{} · Noms</title><style>{}</style></head><body><main>", escape(title), include_str!("../assets/recipe-export.css")).unwrap();
        out.push_str("<header class=\"masthead\"><div class=\"brand\"><svg viewBox=\"0 0 36 40\" aria-hidden=\"true\" xmlns=\"http://www.w3.org/2000/svg\"><rect x=\"4\" y=\"3\" width=\"27\" height=\"34\" rx=\"4\" fill=\"#647447\"/><path d=\"M8 4v31h22\" fill=\"none\" stroke=\"#c4bd91\" stroke-width=\"2\"/><rect x=\"12\" y=\"12\" width=\"14\" height=\"10\" rx=\"2\" fill=\"#f5ebd6\"/><path d=\"M26 3v30l4 4V3\" fill=\"#d9735a\"/></svg>Noms</div><span class=\"offline-label\">Your offline reading copy</span></header>");
        out
    }

    pub fn footer(images_requested: bool) -> String {
        let mut out = String::new();
        out.push_str("<footer class=\"footnote\"><p>A reading copy from Noms · Open, share, or print this file without an internet connection.</p><p>This copy contains current saved recipes. Editing, history, and re-import require Noms and its JSON format.</p>");
        out.push_str(if images_requested { "<p>Included images are embedded originals; Noms crops and adjustments are not applied.</p>" } else { "<p>Text-only export. Recipe and step images were left out.</p>" });
        out.push_str("</footer></main></body></html>");
        out
    }

    pub fn article(
        recipe: &Recipe,
        tags: &[String],
        embedded: &HashMap<String, String>,
        warnings: &[String],
        index: Option<usize>,
    ) -> String {
        let mut out = String::new();
        let section_heading = if index.is_some() { "h3" } else { "h2" };
        if let Some(index) = index {
            write!(out, "<article id=\"recipe-{index}\" class=\"cookbook-recipe\"><a class=\"back-link\" href=\"#contents\">Back to contents</a>").unwrap();
        } else {
            out.push_str("<article>");
        }
        out.push_str("<header class=\"card hero\"><span class=\"eyebrow\">From your recipe collection</span>");
        let heading = if index.is_some() { "h2" } else { "h1" };
        write!(
            out,
            "<{heading} class=\"recipe-title\">{}</{heading}>",
            escape(&recipe.title)
        )
        .unwrap();
        paragraph(
            &mut out,
            recipe.description.as_deref(),
            "description authored",
        );
        if !tags.is_empty() {
            out.push_str("<div class=\"tags\" aria-label=\"Recipe tags\">");
            for tag in tags {
                write!(out, "<span class=\"tag\">{}</span>", escape(tag)).unwrap();
            }
            out.push_str("</div>");
        }
        if recipe.prep_time_minutes.is_some()
            || recipe.cook_time_minutes.is_some()
            || recipe.servings.is_some()
        {
            out.push_str("<dl class=\"meta\">");
            for (name, value, suffix) in [
                ("Prep", recipe.prep_time_minutes, " min"),
                ("Cook", recipe.cook_time_minutes, " min"),
                ("Serves", recipe.servings, ""),
            ] {
                if let Some(value) = value {
                    write!(out, "<div><dt>{name}</dt><dd>{value}{suffix}</dd></div>").unwrap();
                }
            }
            out.push_str("</dl>");
        }
        write!(
            out,
            "<p class=\"byline\">Recipe by {}</p>",
            escape(&recipe.author_username)
        )
        .unwrap();
        photos(&mut out, &recipe.images, embedded, "Recipe");
        out.push_str("</header>");
        if !warnings.is_empty() {
            write!(out, "<aside class=\"notice\"><{section_heading}>Some images are not in this copy</{section_heading}><ul>").unwrap();
            for warning in warnings {
                write!(out, "<li>{}</li>", escape(warning)).unwrap();
            }
            out.push_str("</ul></aside>");
        }
        out.push_str("<div class=\"columns\"><div>");
        if !recipe.ingredients.is_empty() {
            write!(out, "<section class=\"card\"><{section_heading} class=\"section-title\">Ingredients</{section_heading}><ul class=\"ingredients\">").unwrap();
            for ingredient in &recipe.ingredients {
                let quantity = format!("{} {}", ingredient.amount, ingredient.unit);
                write!(
                    out,
                    "<li><span class=\"quantity\">{}</span> {}</li>",
                    escape(quantity.trim()),
                    escape(&ingredient.name)
                )
                .unwrap();
            }
            out.push_str("</ul></section>");
        }
        if !recipe.equipment.is_empty() {
            write!(out, "<section class=\"card\"><{section_heading} class=\"section-title\">Equipment</{section_heading}><ul class=\"equipment\">").unwrap();
            for item in &recipe.equipment {
                write!(out, "<li>{}</li>", escape(&item.name)).unwrap();
            }
            out.push_str("</ul></section>");
        }
        out.push_str("</div><div>");
        if !recipe.instructions.is_empty() {
            write!(out, "<section class=\"card\"><{section_heading} class=\"section-title\">Method</{section_heading}>").unwrap();
            steps(&mut out, &recipe.instructions, embedded, "");
            out.push_str("</section>");
        }
        out.push_str("</div></div>");
        if recipe.commentary.as_ref().is_some_and(|s| !s.is_empty()) {
            write!(out, "<section class=\"card notes\"><{section_heading} class=\"section-title\">Recipe notes</{section_heading}>").unwrap();
            paragraph(&mut out, recipe.commentary.as_deref(), "authored");
            out.push_str("</section>");
        }
        out.push_str("</article>");
        out
    }

    pub fn document(
        recipe: &Recipe,
        tags: &[String],
        embedded: &HashMap<String, String>,
        warnings: &[String],
        images_requested: bool,
    ) -> String {
        let mut out = start_document(&recipe.title);
        out.push_str(&article(recipe, tags, embedded, warnings, None));
        out.push_str(&footer(images_requested));
        out
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::render::*;
    #[tokio::test]
    async fn html_is_escaped_and_stays_inert_for_hostile_authored_text() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let storage = crate::storage::StorageClient::for_test();
        let (mut recipe, _) = crate::api::recipe_archive::tests::source(&pool, &storage).await;
        recipe.title = "</title><script>alert(1)</script>".into();
        recipe.description = Some("<img src=x onerror=alert(1)> & \"hello\"".into());
        recipe.commentary = Some("</style><iframe src=https://evil.example></iframe>".into());
        let html = document(
            &recipe,
            &["<svg onload=alert(1)>".into()],
            &Default::default(),
            &[],
            false,
        );
        assert!(
            !html.contains("<script>") && !html.contains("<iframe") && !html.contains("<img src=x")
        );
        assert!(html.contains("&lt;script&gt;") && html.contains("&amp;"));
        assert!(html.contains("form-action 'none'") && html.contains("base-uri 'none'"));
        assert!(html.contains("Cook") && html.contains("Stir"));
        assert!(!html.contains("src=\"https:"));
    }
}
