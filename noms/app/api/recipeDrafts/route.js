import axios from "axios";
import { getServerSession } from "next-auth";
import { authOptions } from "../auth/[...nextauth]/route";

export async function POST(req) {
  const session = await getServerSession(authOptions);
  if (!session) {
    return Response.json({ error: "Unauthorized" }, { status: 401 });
  }

  const data = await req.json();
  const url = data.url?.trim();

  if (!url) {
    return Response.json({ error: "URL is required" }, { status: 400 });
  }

  return axios
    .post(
      process.env.LAMBDA_API_URL,
      {
        sql: `
          INSERT INTO recipe_drafts (userid, url)
          VALUES (%s, %s)
        `,
        values: [session.user.id, url],
      },
      {
        headers: {
          "Content-Type": "application/json",
          "x-api-key": process.env.LAMBDA_API_KEY,
        },
      }
    )
    .then((response) => {
      return Response.json(response.data, { status: response.status });
    })
    .catch((error) => {
      return Response.json(error.response?.data || { error: error.message }, {
        status: error.response?.status || 500,
      });
    });
}
