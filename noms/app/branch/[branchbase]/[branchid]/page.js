import { Typography } from "@mui/material";


export async function generateStaticParams() {
    const recipeIDs = ['1']
    return recipeIDs.map((id) => {
      return {recipeID: id}
    });
}

async function getBranchRecipes(branchbase, branchid) {
    return fetch(
        `${process.env.NOMS_URL}/api/getRecipeBranch?branchbase=${branchbase}&branchid=${branchid}`
    ).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

export default async function Recipe({ params }) {
    const branchRecipes = await getBranchRecipes(params.branchbase, params.branchid)

    return (
        <Typography>{JSON.stringify(branchRecipes)}</Typography>
    )
}