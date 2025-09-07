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
                SELECT DISTINCT baseid
                FROM recipes;
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((base) => {
            return base.baseid.toString()
        })
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getTreeRecipes(baseid) {
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
            FROM 
                recipes r
            JOIN 
                (
                    SELECT 
                        branchid,
                        branchbase, 
                        baseid, 
                        MAX(version) AS max_version
                    FROM 
                        recipes
                    WHERE
                        baseid = %s
                    GROUP BY 
                        branchid, branchbase, baseid
                ) latest
            ON 
                r.branchid = latest.branchid 
                AND r.branchbase = latest.branchbase 
                AND r.baseid = latest.baseid 
                AND r.version = latest.max_version
            LEFT JOIN users u ON r.userid = u.id
            LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
            LEFT JOIN ingredients i ON ri.ingredientid = i.id
            GROUP BY r.id, u.name
            ORDER BY r.branchbase ASC, r.branchid ASC
            `,
            values: [baseid]
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
        return {message: 'error'}
    })
}

export default async function Recipe({ params }) {
    const treeRecipes = await getTreeRecipes(params.baseid)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.titleSize, marginLeft: '150px'}}>Tree</Typography>
                <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
                    {treeRecipes.map((recipe, index) => { 
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