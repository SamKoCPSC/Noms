import axios from "axios"
import { getServerSession } from "next-auth"
import { authOptions } from "../auth/[...nextauth]/route"
import { revalidatePath } from "next/cache"

export async function POST(req, res) {
    const session = await getServerSession(authOptions)
    const data = await req.json()

    const name = data.name
    const description = data.description || null
    const items = data.items || [] 
    // items = [{ type: "recipe", id: 123, position: 1 }, { type: "collection", id: 456, position: 2 }]

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                WITH new_collection AS (
                    INSERT INTO collections (name, description, userid)
                    VALUES (%s, %s, %s)
                    RETURNING id
                )
                ${items.length > 0 ? `
                , inserted_items AS (
                    INSERT INTO collection_items (parent_collection_id, item_type, recipe_id, child_collection_id, position)
                    SELECT 
                        new_collection.id,
                        i.item_type,
                        CASE WHEN i.item_type = 'recipe' THEN i.item_id ELSE NULL END,
                        CASE WHEN i.item_type = 'collection' THEN i.item_id ELSE NULL END,
                        i.position
                    FROM new_collection,
                         (VALUES ${items.map((_, idx) => 
                            `(%s, %s, %s)`
                         ).join(", ")}) 
                         AS i(item_type, item_id, position)
                    RETURNING id
                )
                SELECT new_collection.id FROM new_collection
                ` : `
                SELECT id FROM new_collection
                `}
            `,
            values: [
                name, description, session.user.id,
                ...items.flatMap(i => [i.type, i.id, i.position])
            ]
        },
        {
            headers: {
                "Content-Type": "application/json",
                "x-api-key": process.env.LAMBDA_API_KEY,
            }
        }
    )
    .then((response) => {
        const newCollectionId = response.data.result[0].id

        revalidatePath(`/myCollections/${session.user.id}`)
        revalidatePath(`/collection/${newCollectionId}`)

        return Response.json(
            response.data,
            { status: response.status }
        )
    })
    .catch((error) => {
        return Response.json(
            error.response?.data || { error: "Failed to create collection" },
            { status: error.response?.status || 500 }
        )
    })
}
