import axios from "axios";
import { revalidatePath } from "next/cache";

export async function POST(req, res) {
    const data = await req.json()
    const name = data.name
    const email = data.email

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
            INSERT INTO users (name, email)
            VALUES (%s, %s)
            ON CONFLICT (email) DO UPDATE set email = EXCLUDED.email
            RETURNING id
            `,
            values: [name, email]
        },
        {
            headers: {
                'Content-Type': 'application/json',
                'x-api-key': process.env.LAMBDA_API_KEY,
            }
        }
    ).then((response) => {
        revalidatePath(`/account/${response.data.result[0].id}`)
        revalidatePath(`/myRecipes/${response.data.result[0].id}`)
        return Response.json(
            response.data,
            {status: response.status}
        )
    }).catch((error) => {
        return Response.json(
            error.response.data,
            {status: error.response.status}
        )
    })
}