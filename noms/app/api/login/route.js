import axios from "axios";

export async function GET(req, res) {
    const name = req.nextUrl.searchParams.get('name')
    const email = req.nextUrl.searchParams.get('email')

    return axios.get(
        process.env.LAMBDA_API_URL,
        {
            params: {
                sql: `
                    INSERT INTO users (name, email)
                    VALUES ('${name}', '${email}')
                    ON CONFLICT (email) DO NOTHING
                `,
            },
            headers: {
                'Content-Type': 'application/json',
                'x-api-key': process.env.LAMBDA_API_KEY,
            }
        }
    ).then((response) => {
        console.log(req)
        console.log(response)
        return Response.json(
            response.data,
            {status: response.status}
        )
    }).catch((error) => {
        console.log(req)
        console.error(error.response)
        return Response.json(
            error.response.data,
            {status: error.response.status}
        )
    })
}