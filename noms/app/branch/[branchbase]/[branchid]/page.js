import { Typography, Container, Divider, Box, Avatar } from "@mui/material";
import RecipeCard from "@/app/components/RecipeCard";

function formatTimestamp(timestamp) {
    const isoTimestamp = timestamp.replace(" ", "T");
    const date = new Date(isoTimestamp);
    if (isNaN(date.getTime())) {
        throw new Error("Invalid PostgreSQL timestamp format.");
    }
    const options = {
        year: "numeric",
        month: "long",
        day: "numeric",
    };
    return date.toLocaleDateString(undefined, options);
}

export async function generateStaticParams() {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT DISTINCT branchbase, branchid
                FROM recipes
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((branch) => ({
            branchbase: branch.branchbase.toString(),
            branchid: branch.branchid.toString(),
        }))
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getBranchRecipes(branchbase, branchid) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    r.id AS recipeid,
                    r.name AS name,
                    r.description,
                    r.instructions,
                    r.userid,
                    r.additionalinfo,
                    r.imageurls,
                    r.status,
                    r.datecreated,
                    r.baseid,
                    r.version,
                    r.branchid,
                    r.branchbase,
                    r.notes,
                    u.name AS author,
                    json_agg(
                        json_build_object(
                            'id', i.id,
                            'name', i.name,
                            'quantity', ri.quantity,
                            'unit', ri.unit
                        )
                    ) AS ingredients
                FROM recipes r
                LEFT JOIN users u ON r.userid = u.id
                LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
                LEFT JOIN ingredients i ON ri.ingredientid = i.id
                WHERE r.branchbase = %s AND r.branchid = %s OR r.id = %s
                GROUP BY r.id, u.name
                ORDER BY r.id ASC
            `,
            values: [branchbase, branchid, branchbase]
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

export default async function Recipe({ params }) {
    const branchRecipes = await getBranchRecipes(params.branchbase, params.branchid)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.titleSize, marginLeft: '150px'}}>Branch</Typography>
                <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
                    {branchRecipes.map((recipe, index) => { 
                        if(recipe.status === 'public') {
                            return (
                                <RecipeCard
                                    key={index}
                                    id={recipe.recipeid}
                                    name={recipe.name}
                                    description={recipe.description}
                                    author={recipe.author}
                                    date={formatTimestamp(recipe.datecreated)}
                                    ingredients={recipe.ingredients}
                                    instructions={recipe.instructions}
                                    additionalInfo={recipe.additionalinfo}
                                    imageURLs={recipe.imageurls}
                                    status={recipe.status}
                                    baseid={recipe.baseid}
                                    version={recipe.version}
                                    branchid = {recipe.branchid}
                                    branchbase = {recipe.branchbase}
                                />
                            )
                        }  
                    })}
                </Box>
            </Box>
        </Container>
    )
}