'use client'
import { Box, Card, CardContent, CardMedia, Typography } from "@mui/material"

export default function RecipeCardMini({imageURL, name, variantName, ownerName}) {
    return (
        <Card sx={{height: '140px', width: '160px'}}>
            <CardMedia
                component="img"
                height="100"
                image={imageURL ? imageURL : undefined}
                alt="No Image"
            />
            <CardContent sx={{paddingY: 0, paddingX: 0.75}}>
                <Box display={'flex'} flexDirection={'column'}>
                    <Typography sx={{fontSize: '12px'}}>
                        {name}
                    </Typography>
                    <Typography sx={{fontSize: '12px'}}>
                        {ownerName}
                    </Typography>
                </Box>
            </CardContent>
        </Card>
    )
}