import { Typography, Box, Divider } from "@mui/material";


export default function About() {
    return (
        <Box 
            display={'flex'} 
            flexDirection={'column'}
            sx={{
                justifySelf: 'center',
                width: '1000px', 
                marginTop: '100px',
            }}>
            <Typography sx={{fontSize: '80px'}}>About Noms</Typography>
            <Divider sx={{marginY: '30px'}}></Divider>
            <Box sx={{marginX: '20px'}}>
                <Typography sx={{fontSize: '1.5rem'}}>
                    Hi! I'm <b>Sam</b>, the creator of this app.<br/><br/> 
                    I created <b>Noms</b> because as a food and cooking hobbyist I often found myself encountering the same problems repeatedly. The primary problem was I
                    wanted a convenient place to store and keep track of my recipes, including images of the product, the ingredients, and list of instructions. 
                    I used to write down my recipes using Google Docs, but as I developed more recipes over time I found that I would lose track of changes, and the 
                    reasoning behind why I made certain changes.<br/><br/> 
                    Overtime I also discovered other problems and I wished that there would be an easier way to scale recipes accordingly, organize 
                    similar recipes together, figure out what I could make based on the ingredients I had available or figure out what I needed to buy for a particular recipe<br/><br/>
                    And so, I decided to create my own solution. With <b>Noms</b> anyone can create an account, and start archiving, developing, and sharing recipes. 
                    Once a recipe is created, a stylized page for that recipe will be automatically generated. New versions and variations of recipes can be created based on 
                    older recipes, and overtime, a recipe may develop into a tree of recipe iterations.
                </Typography>
            </Box>
        </Box>
    )
}