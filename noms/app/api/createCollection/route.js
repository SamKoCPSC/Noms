import axios from "axios"
import { getServerSession } from "next-auth"
import { authOptions } from "../auth/[...nextauth]/route"
import { revalidatePath } from "next/cache"

export async function POST(req, res) {
    const session = await getServerSession(authOptions)
    const data = await req.json()

    const name = data.name
    const description = data.description || null
    const variantIds = data.variantIds || []

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                WITH new_collection AS (
                    INSERT INTO collections (ownerid, name, description)
                    VALUES (%s, %s, %s)
                    RETURNING id
                ),
                branch_values AS (
                    SELECT unnest(%s::uuid[]) AS branchid
                ),
                insert_branches AS (
                    INSERT INTO collection_branches (collectionid, branchid, position)
                    SELECT nc.id, bv.branchid, row_number() OVER ()
                    FROM new_collection nc
                    JOIN branch_values bv ON TRUE
                )
                SELECT id
                FROM new_collection;
            `,
            values: [
                session.user.id,
                name,
                description,
                variantIds
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
        revalidatePath(`/collection/${session.user.id}`)

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
