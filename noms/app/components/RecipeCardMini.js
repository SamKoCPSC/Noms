'use client'
import { Box, Card, CardContent, CardMedia, Typography } from "@mui/material"
import { keyframes } from '@mui/material/styles';

const shadowPopBr = keyframes`
    0% {
        -webkit-box-shadow: 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9;
                box-shadow: 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9;
        -webkit-transform: translateX(0) translateY(0);
                transform: translateX(0) translateY(0);
        -webkit-transform: scale(1);
                transform: scale(1);
    }
    100% {
        -webkit-box-shadow: 1px 1px 7px #d7d9d9, 2px 2px 7px #d7d9d9, 3px 3px 7px #d7d9d9, 4px 4px 7px #d7d9d9, 5px 5px 7px #d7d9d9, 6px 6px 7px #d7d9d9, 7px 7px 7px #d7d9d9, 8px 8px 7px #d7d9d9;
                box-shadow: 1px 1px 7px #d7d9d9, 2px 2px 7px #d7d9d9, 3px 3px 7px #d7d9d9, 4px 4px 7px #d7d9d9, 5px 5px 7px #d7d9d9, 6px 6px 7px #d7d9d9, 7px 7px 7px #d7d9d9, 8px 8px 7px #d7d9d9;
        -webkit-transform: translateX(-8px) translateY(-8px);
                transform: translateX(-8px) translateY(-8px);
        -webkit-transform: scale(1.05);
                transform: scale(1.05);
    }`


const shadowUnPopBr = keyframes`
    0% {
        -webkit-box-shadow: 1px 1px 7px #d7d9d9, 2px 2px 7px #d7d9d9, 3px 3px 7px #d7d9d9, 4px 4px 7px #d7d9d9, 5px 5px 7px #d7d9d9, 6px 6px 7px #d7d9d9, 7px 7px 7px #d7d9d9, 8px 8px 7px #d7d9d9;
                box-shadow: 1px 1px 7px #d7d9d9, 2px 2px 7px #d7d9d9, 3px 3px 7px #d7d9d9, 4px 4px 7px #d7d9d9, 5px 5px 7px #d7d9d9, 6px 6px 7px #d7d9d9, 7px 7px 7px #d7d9d9, 8px 8px 7px #d7d9d9;
        -webkit-transform: translateX(-8px) translateY(-8px);
                transform: translateX(-8px) translateY(-8px);
        -webkit-transform: scale(1.05);
                transform: scale(1.05);
    }
    100% {
        -webkit-box-shadow: 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9;
                box-shadow: 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9, 0 0 #d7d9d9;
        -webkit-transform: translateX(0) translateY(0);
                transform: translateX(0) translateY(0);
        -webkit-transform: scale(1);
                transform: scale(1);
    }`

export default function RecipeCardMini({name, variantName, ownerName, imageURLs}) {
    return (
        <Card sx={{
            height: '140px', 
            width: '160px',
            animation: `${shadowUnPopBr} 0.15s ease-out both`,
            "&:hover": {
                backgroundColor: '#f0f0f0',
                animation: `${shadowPopBr} 0.15s ease-in both`,
                cursor: 'pointer'
            },
        }}>
            <CardMedia
                component="img"
                height="100"
                image={imageURLs ? imageURLs[0] : undefined}
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