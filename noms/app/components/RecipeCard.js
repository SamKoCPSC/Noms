import * as React from 'react';
import { styled } from '@mui/material/styles';
import Card from '@mui/material/Card';
import CardHeader from '@mui/material/CardHeader';
import CardMedia from '@mui/material/CardMedia';
import CardContent from '@mui/material/CardContent';
import CardActions from '@mui/material/CardActions';
import Collapse from '@mui/material/Collapse';
import Avatar from '@mui/material/Avatar';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import { red } from '@mui/material/colors';
import FavoriteIcon from '@mui/icons-material/Favorite';
import ShareIcon from '@mui/icons-material/Share';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import MoreVertIcon from '@mui/icons-material/MoreVert';
import { ThumbUp } from '@mui/icons-material';
import { useRouter } from 'next/navigation';
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

const ExpandMore = styled((props) => {
  const { expand, ...other } = props;
  return <IconButton {...other} />;
})(({ theme }) => ({
  marginLeft: 'auto',
  transition: theme.transitions.create('transform', {
    duration: theme.transitions.duration.shortest,
  }),
  variants: [
    {
      props: ({ expand }) => !expand,
      style: {
        transform: 'rotate(0deg)',
      },
    },
    {
      props: ({ expand }) => !!expand,
      style: {
        transform: 'rotate(180deg)',
      },
    },
  ],
}));

export default function RecipeCard({title, description, author, date, ingredients, instructions}) {
  const [expanded, setExpanded] = React.useState(false);
  const router = useRouter()

  const handleExpandClick = () => {
    setExpanded(!expanded);
  };

  return (
    <Card 
    sx={{
      href: '/product',
      width: '350px',
      height: '420px',
      backgroundColor: 'white',
      borderRadius: '25px',
      animation: `${shadowUnPopBr} 0.15s ease-out both`,
      "&:hover": {
          backgroundColor: '#f0f0f0',
          animation: `${shadowPopBr} 0.15s ease-in both`,
          cursor: 'pointer'
      },

    }}
    // onMouseEnter={() => {setMouseHover(true)}}
    // onMouseLeave={() => {setMouseHover(false)}}
    onClick = {() => router.push('/recipe')}
  >
      <CardHeader
        avatar={
          <Avatar sx={{ bgcolor: red[500] }} aria-label="recipe">
            R
          </Avatar>
        }
        action={
          <IconButton aria-label="settings">
            <MoreVertIcon />
          </IconButton>
        }
        titleTypographyProps={{fontSize:'20px' }}
        subheaderTypographyProps={{fontSize:'12px' }}
        title={title}
        subheader={author + ' - ' + date}
      />
      <CardMedia
        component="img"
        height="194"
        image="/croissant1.jpg"
        src='img'
        alt="Image not available"
      />
      <CardContent>
        <Typography variant="body2" sx={{ color: 'text.secondary' }}>
          {description.substring(0,175)}{description.length >= 175 ? "..." : ""}
        </Typography>
      </CardContent>
      <CardActions disableSpacing>
        <IconButton aria-label="add to favorites" 
          onClick={(e) => {
            e.stopPropagation()
            router.push('/create')
          }}
        >
          <FavoriteIcon />
        </IconButton>
        <IconButton aria-label='like'>
          <ThumbUp/>
        </IconButton>
        <IconButton aria-label="share">
          <ShareIcon />
        </IconButton>
        {/* <ExpandMore
          expand={expanded}
          onClick={handleExpandClick}
          aria-expanded={expanded}
          aria-label="show more"
        >
          <ExpandMoreIcon />
        </ExpandMore> */}
      </CardActions>
      <Collapse in={expanded} timeout="auto" unmountOnExit>
        <CardContent>
          <Typography sx={{ marginBottom: 2 }}>Method:</Typography>
          <Typography sx={{ marginBottom: 2 }}>
            Heat 1/2 cup of the broth in a pot until simmering, add saffron and set
            aside for 10 minutes.
          </Typography>
          <Typography sx={{ marginBottom: 2 }}>
            Heat oil in a (14- to 16-inch) paella pan or a large, deep skillet over
            medium-high heat. Add chicken, shrimp and chorizo, and cook, stirring
            occasionally until lightly browned, 6 to 8 minutes. Transfer shrimp to a
            large plate and set aside, leaving chicken and chorizo in the pan. Add
            pimentón, bay leaves, garlic, tomatoes, onion, salt and pepper, and cook,
            stirring often until thickened and fragrant, about 10 minutes. Add
            saffron broth and remaining 4 1/2 cups chicken broth; bring to a boil.
          </Typography>
          <Typography sx={{ marginBottom: 2 }}>
            Add rice and stir very gently to distribute. Top with artichokes and
            peppers, and cook without stirring, until most of the liquid is absorbed,
            15 to 18 minutes. Reduce heat to medium-low, add reserved shrimp and
            mussels, tucking them down into the rice, and cook again without
            stirring, until mussels have opened and rice is just tender, 5 to 7
            minutes more. (Discard any mussels that don&apos;t open.)
          </Typography>
          <Typography>
            Set aside off of the heat to let rest for 10 minutes, and then serve.
          </Typography>
        </CardContent>
      </Collapse>
    </Card>
  );
}
