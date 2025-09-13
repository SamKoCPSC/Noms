import { Typography, Container, Box, Button} from "@mui/material";
import RecipeCard from "@/app/components/RecipeCard";
import formatTimestamp from "@/app/function/formatTimestamp";
import Link from "next/link";

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

async function getBranchRecipes(branchid) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    b.id,
                    b.name,
                    b.description,
                    b.ownerid,
                    u.name AS ownername,
                    b.baserecipeid,
                    b.headrecipeid,
                    b.projectid,
                    b.created_at,
                    (
                        SELECT json_agg(
                            json_build_object(
                                'id', r.id,
                                'name', r.name,
                                'description', r.description,
                                'status', r.status,
                                'author', ru.name,
                                'datecreated', r.datecreated,
                                'instructions', r.instructions,
                                'additionalinfo', r.additionalinfo,
                                'notes', r.notes,
                                'imageurls', r.imageurls,
                                'position', rb.position,
                                'ingredients', (
                                    SELECT json_agg(
                                        json_build_object(
                                            'name', i.name,
                                            'quantity', ri.quantity,
                                            'unit', ri.unit
                                        ) ORDER BY i.name
                                    )
                                    FROM recipe_ingredients ri
                                    JOIN ingredients i ON ri.ingredientid = i.id
                                    WHERE ri.recipeid = r.id
                                )
                            ) ORDER BY rb.position
                        )
                        FROM recipe_branches rb
                        JOIN recipes r ON rb.recipeid = r.id
                        JOIN users ru ON r.userid = ru.id
                        WHERE rb.branchid = b.id
                    ) AS recipes,
                    (
                        SELECT json_build_object(
                            'id', r.id,
                            'name', r.name,
                            'description', r.description,
                            'status', r.status,
                            'author', ru.name,
                            'datecreated', r.datecreated,
                            'imageurls', r.imageurls
                        )
                        FROM projects p
                        JOIN branches root_b ON root_b.projectid = p.id
                        JOIN recipes r ON root_b.baserecipeid = r.id
                        JOIN users ru ON r.userid = ru.id
                        WHERE p.id = b.projectid
                        ORDER BY root_b.created_at ASC
                        LIMIT 1
                    ) AS original
                FROM branches b
                JOIN users u ON b.ownerid = u.id
                WHERE b.id = %s
                GROUP BY b.id, u.name;

            `,
            values: [branchid]
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
    const variant = await getBranchRecipes(params.branchid)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    const latestVersion = variant[0]?.recipes?.length ? variant[0].recipes[variant[0].recipes.length - 1] : null
    const initialVersion = variant[0]?.recipes?.length ? variant[0].recipes[0] : null
    const originalVersion = variant[0]?.original

    return (
        <Container sx={{justifyItems: 'center', width: '100%'}}>
            <Box 
                display="flex"
                alignItems="flex-start"
                sx={{
                    width: '100%',
                    backgroundColor: 'white',
                    padding: '20px',
                    margin: '30px',
                    borderRadius: '15px',
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
                }}
            >
                <Box sx={{ flex: 1 }}>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.sectionTitleSize,
                            marginBottom: '0px',
                            textAlign: 'left'
                        }}
                    >
                        {variant[0]?.name || 'Variation Name'}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: '0.9rem',
                            marginBottom: '10px',
                            textAlign: 'left'
                        }}
                    >
                        By: {variant[0]?.ownername || 'Variant Owner'} | Created: {formatTimestamp(variant[0]?.created_at)}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.paragraphSize,
                            textAlign: 'left',
                            lineHeight: 1.5
                        }}
                    >
                        {variant[0]?.description || 'No description available'}
                    </Typography>
                </Box>
                <Box 
                    display="flex" 
                    flexDirection="row" 
                    alignItems="flex-end"
                    sx={{ gap: '15px' }}
                >
                    <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
                        <Typography 
                            variant="h4" 
                            sx={{ 
                                fontSize: textStyle.sectionTitleSize,
                                fontWeight: 'bold',
                                color: 'secondary.main'
                            }}
                        >
                            {variant[0]?.recipes.length || 0}
                        </Typography>
                        <Typography 
                            variant="body2" 
                            sx={{ 
                                fontSize: textStyle.paragraphSize,
                                color: 'text.secondary'
                            }}
                        >
                            Versions
                        </Typography>
                        <Link href={`/createRecipe?name=${latestVersion.name}&description=${latestVersion.description}&ingredients=${JSON.stringify(latestVersion.ingredients)}&instructions=${JSON.stringify(latestVersion.instructions)}&additionalInfo=${JSON.stringify(latestVersion.additionalinfo)}&imageURLs=${JSON.stringify(latestVersion.imageurls)}&branchid=${variant[0].id}`}>
                            <Button variant="contained">
                                Create New Version
                            </Button>
                        </Link>
                    </Box>
                </Box>
            </Box>
            {latestVersion && (
                <Box
                    display='flex'
                    sx={{
                        width: '100%',
                        backgroundColor: 'white',
                        padding: '20px',
                        margin: '30px',
                        borderRadius: '15px',
                        borderColor: 'rgb(230, 228, 215)',
                        borderStyle: 'solid',
                        borderWidth: 2,
                        boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
                        justifyContent: 'space-evenly'
                    }}
                >
                    <Box>
                        <Typography
                            sx={{
                                fontSize: textStyle.sectionTitleSize,
                                marginBottom: '20px',
                                textAlign: 'left'
                            }}
                        >
                            Latest Version
                        </Typography>
                        <Box display="flex" justifyContent="flex-start">
                            <RecipeCard
                                id={latestVersion.id}
                                name={latestVersion.name}
                                description={latestVersion.description}
                                author={latestVersion.author || (variant[0]?.ownername ?? 'Unknown')}
                                date={formatTimestamp(latestVersion.datecreated)}
                                imageURLs={latestVersion.imageurls}
                                status={latestVersion.status}
                            />
                        </Box>
                    </Box>
                    <Box>
                        <Typography
                            sx={{
                                fontSize: textStyle.sectionTitleSize,
                                marginBottom: '20px',
                                textAlign: 'left'
                            }}
                        >
                            Initial Version
                        </Typography>
                        <Box display="flex" justifyContent="flex-start">
                            <RecipeCard
                                id={initialVersion.id}
                                name={initialVersion.name}
                                description={initialVersion.description}
                                author={initialVersion.author || (variant[0]?.ownername ?? 'Unknown')}
                                date={formatTimestamp(initialVersion.datecreated)}
                                imageURLs={initialVersion.imageurls}
                                status={initialVersion.status}
                            />
                        </Box>
                    </Box>
                    <Box>
                        <Typography
                            sx={{
                                fontSize: textStyle.sectionTitleSize,
                                marginBottom: '20px',
                                textAlign: 'left'
                            }}
                        >
                            Original Version
                        </Typography>
                        <Box display="flex" justifyContent="flex-start">
                            <RecipeCard
                                id={originalVersion.id}
                                name={originalVersion.name}
                                description={originalVersion.description}
                                author={originalVersion.author || (variant[0]?.ownername ?? 'Unknown')}
                                date={formatTimestamp(originalVersion.datecreated)}
                                imageURLs={originalVersion.imageurls}
                                status={originalVersion.status}
                            />
                        </Box>
                    </Box>
                </Box>
                
            )}
            <Box 
                display="flex"
                flexDirection={'column'}
                alignItems="flex-start"
                sx={{
                    width: '100%',
                    backgroundColor: 'white',
                    paddingTop: '20px',
                    paddingBottom: '5px',
                    margin: '30px',
                    borderRadius: '15px',
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
                }}
            >
                <Typography
                    sx={{ 
                        fontSize: '1.7rem',
                        textAlign: 'left',
                        lineHeight: 1.5,
                        marginLeft: '20px',
                        marginBottom: '10px',
                    }}
                >
                    Version History
                </Typography>
                {variant[0]?.recipes.map((recipe) => {
                    return (
                        <Box key={recipe.id} sx={{
                            width: '100%',
                            paddingRight: '20px', 
                            borderTopStyle: 'solid', 
                            borderTopWidth: 1,
                            transition: 'background-color 0.15s ease-in-out',
                            '&:hover': {
                                backgroundColor: 'rgba(0,0,0,0.08)', // or any theme color
                            },
                        }}>
                            <Link href={`/recipe/${recipe.id}`}>
                                <Box display={'flex'} flexDirection={'row'} sx={{width: '100%'}}>
                                    <Box 
                                        component="img"
                                        src={recipe.imageurls[0] || "/fallback.png"}
                                        alt={`${recipe.name} preview`}
                                        sx={{
                                            width: '160px',
                                            height: '90px',
                                            objectFit: "cover",
                                            marginRight: '5px'
                                        }}
                                    />
                                    <Box display={'flex'} flexDirection={'column'} sx={{flex: 1, minWidth: 0}}>
                                        <Typography sx={{
                                            fontSize: '1.3rem',
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {recipe.name}
                                        </Typography>
                                        <Typography sx={{fontSize: '0.9rem', marginBottom: '10px'}}>Created: {formatTimestamp(recipe.datecreated)}</Typography>
                                        <Typography sx={{
                                            fontSize: '0.9rem', 
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {recipe.description}
                                        </Typography>

                                    </Box>
                                    <Box 
                                        display="flex" 
                                        flexDirection="row" 
                                        alignItems="flex-end"
                                        sx={{ gap: '15px' }}
                                    >
                                        {/* <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
                                            <Typography 
                                                variant="h4" 
                                                sx={{ 
                                                    fontSize: textStyle.sectionTitleSize,
                                                    fontWeight: 'bold',
                                                    color: 'secondary.main'
                                                }}
                                            >
                                                {recipe.recipes?.length || 0}
                                            </Typography>
                                            <Typography 
                                                variant="body2" 
                                                sx={{ 
                                                    fontSize: textStyle.paragraphSize,
                                                    color: 'text.secondary'
                                                }}
                                            >
                                                Recipes
                                            </Typography>
                                        </Box> */}
                                    </Box>
                                </Box>
                            </Link>
                        </Box>
                    )
                })}
            </Box>
        </Container>
    )
}