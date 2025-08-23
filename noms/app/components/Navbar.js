'use client'
import * as React from 'react';
import { useRouter } from 'next/navigation';
import { styled, alpha } from '@mui/material/styles';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Toolbar from '@mui/material/Toolbar';
import IconButton from '@mui/material/IconButton';
import Typography from '@mui/material/Typography';
import InputBase from '@mui/material/InputBase';
import Badge from '@mui/material/Badge';
import MenuItem from '@mui/material/MenuItem';
import Menu from '@mui/material/Menu';
import Modal from '@mui/material/Modal';
import Chip from '@mui/material/Chip';
import MenuIcon from '@mui/icons-material/Menu';
import { Search } from '@mui/icons-material';
import AccountCircle from '@mui/icons-material/AccountCircle';
import MailIcon from '@mui/icons-material/Mail';
import { Face } from '@mui/icons-material';
import NotificationsIcon from '@mui/icons-material/Notifications';
import { Tune } from '@mui/icons-material';
import MoreIcon from '@mui/icons-material/MoreVert';
import { Dancing_Script } from "next/font/google";
import { signIn, signOut, useSession } from "next-auth/react"
import Navdrawer from './Navdrawer'
import { Avatar, TextField, InputAdornment, Button } from '@mui/material';
import theme from '../theme';
import { useTheme } from '@emotion/react';


const dancingScript = Dancing_Script({subsets: ['latin']})

// const Search = styled('div')(({ theme }) => ({
//   position: 'relative',
//   borderRadius: theme.shape.borderRadius,
//   backgroundColor: alpha(theme.palette.common.black, 0.15),
//   '&:hover': {
//     backgroundColor: alpha(theme.palette.common.black, 0.25),
//   },
//   marginRight: theme.spacing(2),
//   marginLeft: 0,
//   width: '100%',
//   [theme.breakpoints.up('sm')]: {
//     marginLeft: theme.spacing(3),
//     width: 'auto',
//   },
// }));

const SearchIconWrapper = styled('div')(({ theme }) => ({
  padding: theme.spacing(0, 2),
  height: '100%',
  position: 'absolute',
  pointerEvents: 'none',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
}));

const StyledInputBase = styled(InputBase)(({ theme }) => ({
  color: 'inherit',
  '& .MuiInputBase-input': {
    padding: theme.spacing(1, 1, 1, 0),
    // vertical padding + font size from searchIcon
    paddingLeft: `calc(1em + ${theme.spacing(4)})`,
    transition: theme.transitions.create('width'),
    width: '100%',
    [theme.breakpoints.up('md')]: {
      width: '20ch',
    },
  },
}));

export default function PrimarySearchAppBar(props) {
  const theme = useTheme()
  const session = useSession()
  const user = session.data?.user

  const [anchorEl, setAnchorEl] = React.useState(null);
  const [mobileMoreAnchorEl, setMobileMoreAnchorEl] = React.useState(null);
  const [isNavdrawerOpen, setNavdrawerOpen] = React.useState(false)
  const [isFilterOpen, setFilterOpen] = React.useState(false)
  const [includedIngredients, setIncludedIngredients] = React.useState([])
  const [excludedIngredients, setExcludedIngredients] = React.useState([])
  const [navbarHeight, setNavbarHeight] = React.useState(56)
  const appBarRef = React.useRef(null)


  const router = useRouter()

  const isMenuOpen = Boolean(anchorEl);
  const isMobileMenuOpen = Boolean(mobileMoreAnchorEl);

  // Measure navbar height dynamically
  React.useEffect(() => {
    const measureHeight = () => {
      if (appBarRef.current) {
        const height = appBarRef.current.offsetHeight;
        setNavbarHeight(height);
      }
    };

    // Measure height after render
    measureHeight();

    // Add resize listener to handle responsive changes
    window.addEventListener('resize', measureHeight);
    
    // Cleanup
    return () => window.removeEventListener('resize', measureHeight);
  }, [isFilterOpen, includedIngredients, excludedIngredients]); // Re-measure when filter state or ingredients change

  const handleProfileMenuOpen = (event) => {
    setAnchorEl(event.currentTarget);
  };

  const handleMobileMenuClose = () => {
    setMobileMoreAnchorEl(null);
  };

  const handleMenuClose = () => {
    setAnchorEl(null);
    handleMobileMenuClose();
  };

  const handleMobileMenuOpen = (event) => {
    setMobileMoreAnchorEl(event.currentTarget);
  };

  const handleNavdrawerOpen = () => {
    setNavdrawerOpen(!isNavdrawerOpen)
  }

  const handleSearch = (event) => {
    event.preventDefault()
    const formData = new FormData(event.currentTarget)
    router.push(`/search?name=${formData.get('name')}&includedIngredients=${JSON.stringify(includedIngredients)}&excludedIngredients=${JSON.stringify(excludedIngredients)}`)
  }

  const menuId = 'primary-search-account-menu';
  const renderMenu = (
    <Menu
      anchorEl={anchorEl}
      anchorOrigin={{
        vertical: 'top',
        horizontal: 'right',
      }}
      id={menuId}
      keepMounted
      transformOrigin={{
        vertical: 'top',
        horizontal: 'right',
      }}
      open={isMenuOpen}
      onClose={handleMenuClose}
    >
      {user ? 
        <MenuItem onClick={() => signOut({callbackUrl: '/'})}>Logout</MenuItem> 
        :
        <MenuItem onClick={() => signIn('google')}>Login</MenuItem>}
    </Menu>
  );

  const mobileMenuId = 'primary-search-account-menu-mobile';
  const renderMobileMenu = (
    <Menu
      anchorEl={mobileMoreAnchorEl}
      anchorOrigin={{
        vertical: 'top',
        horizontal: 'right',
      }}
      id={mobileMenuId}
      keepMounted
      transformOrigin={{
        vertical: 'top',
        horizontal: 'right',
      }}
      open={isMobileMenuOpen}
      onClose={handleMobileMenuClose}
    >
      <MenuItem>
        <IconButton size="large" aria-label="show 4 new mails" color="inherit">
          <Badge badgeContent={4} color="error">
            <MailIcon />
          </Badge>
        </IconButton>
        <p>Messages</p>
      </MenuItem>
      <MenuItem>
        <IconButton
          size="large"
          aria-label="show 17 new notifications"
          color="inherit"
        >
          <Badge badgeContent={17} color="error">
            <NotificationsIcon />
          </Badge>
        </IconButton>
        <p>Notifications</p>
      </MenuItem>
      <MenuItem onClick={handleProfileMenuOpen}>
        <IconButton
          size="large"
          aria-label="account of current user"
          aria-controls="primary-search-account-menu"
          aria-haspopup="true"
          color="inherit"
        >
          <AccountCircle />
        </IconButton>
        <p>Profile</p>
      </MenuItem>
    </Menu>
  );

  const contentColor = 'black'

  return (
    <Box sx={{ flexGrow: 1, marginBottom: '60px' }}>
      <Navdrawer open={isNavdrawerOpen} setOpen={handleNavdrawerOpen} navbarHeight={navbarHeight}></Navdrawer>
      <AppBar ref={appBarRef} position="fixed" sx={{ zIndex: (theme) => theme.zIndex.drawer + 1, bgcolor: '#d9d7ce', color: 'black' }}>
        <Toolbar>
          <IconButton
            size="large"
            edge="start"
            color="inherit" 
            aria-label="open drawer"
            sx={{ mr: 2, color: contentColor }}
            onClick={() => {handleNavdrawerOpen()}}
          >
            <MenuIcon />
          </IconButton>
          <Typography
            variant="h6"
            noWrap
            component="div"
            sx={{ 
              marginRight: '30px', 
              fontSize: '30px', 
              color: contentColor, 
              display: { sm: 'block', fontFamily: dancingScript.style.fontFamily, ":hover": {cursor: 'pointer'} },
              [theme.breakpoints.down('544')]: {display: 'none'},
            }}
            onClick={() => router.push('/')}
          >
            NOMS
          </Typography>
          <form onSubmit={handleSearch} style={{flexGrow: 1}}>
            <TextField
              name="name"
              variant="outlined"
              placeholder="Search for recipes"
              InputProps={{
                style: {
                  backgroundColor: 'rgb(255, 255, 255)'
                },
                startAdornment: (
                  <InputAdornment position="start">
                    <IconButton onClick={() => setFilterOpen(!isFilterOpen)}>
                      <Tune />
                    </IconButton>
                  </InputAdornment>
                ),
                endAdornment: (
                  <InputAdornment position="end">
                    <IconButton type="submit">
                      <Search />
                    </IconButton>
                  </InputAdornment>
                ),
              }}
              sx={{
                flexGrow: 1,
                '& .MuiOutlinedInput-root': {
                  borderRadius: '25px',
                },
                width: '500px',
                [theme.breakpoints.down('760')]: {width: '100%'},
              }}
            />
          </form>
          {/* <Search sx={{color: contentColor}}>
            <SearchIconWrapper>
              <SearchIcon sx={{color: contentColor}}/>
            </SearchIconWrapper>
            <StyledInputBase
              placeholder="Search for recipes"
              inputProps={{ 'aria-label': 'search' }}
            />
          </Search> */}
          {/* <Box sx={{ flexGrow: 1}}/> */}
          <Box sx={{ display: 'flex' }}>
            {/* <IconButton size="large" aria-label="show 4 new mails" color="inherit">
              <Badge badgeContent={4} color="error">
                <MailIcon sx={{color: contentColor}} />
              </Badge>
            </IconButton>
            <IconButton
              size="large"
              aria-label="show 17 new notifications"
              color="inherit"
            >
              <Badge badgeContent={17} color="error">
                <NotificationsIcon sx={{color: contentColor}}/>
              </Badge>
            </IconButton> */}
            <IconButton
              size="large"
              edge="end"
              aria-label="account of current user"
              aria-controls={menuId}
              aria-haspopup="true"
              onClick={handleProfileMenuOpen}
              color="inherit"
            >
              {user ? <Avatar sx={{bgcolor: theme.palette.primary.main}}>{user.firstName.charAt(0)+user.lastName.charAt(0)}</Avatar> : <AccountCircle sx={{color: contentColor}}/>}
            </IconButton>
          </Box>
          {/* <Box sx={{ display: { xs: 'flex', md: 'none' } }}>
            <IconButton
              size="large"
              aria-label="show more"
              aria-controls={mobileMenuId}
              aria-haspopup="true"
              onClick={handleMobileMenuOpen}
              color="inherit"
            >
              <MoreIcon />
            </IconButton>
          </Box> */}
        </Toolbar>
        {isFilterOpen && (
          <Box sx={{ 
            bgcolor: '#d9d7ce', 
            color: 'black', 
            px: 3, 
            py: 1,
            borderTop: '1px solid rgba(0, 0, 0, 0.12)'
          }}>
            <Box display="flex" flexDirection="row" alignItems="center" gap={3} flexWrap="wrap">
              {/* Included Ingredients Section */}
              <Box display="flex" alignItems="center" gap={1}>
                <Typography variant="body2" sx={{ fontWeight: 'bold', minWidth: 'fit-content' }}>
                  Include:
                </Typography>
                <form 
                  onSubmit={(event) => {
                    event.preventDefault()
                    const formData = new FormData(event.currentTarget)
                    const newIngredient = formData.get('includedIngredients')
                    if (newIngredient && newIngredient.trim()) {
                      setIncludedIngredients([...includedIngredients, newIngredient.trim()])
                      event.target.reset()
                    }
                  }}
                >
                  <TextField
                    name="includedIngredients"
                    variant="outlined"
                    placeholder="Add ingredient"
                    size="small"
                    sx={{
                      '& .MuiOutlinedInput-root': {
                        borderRadius: '15px',
                        bgcolor: 'white',
                        height: '32px'
                      },
                      width: '130px'
                    }}
                  />
                </form>
                <Box display="flex" flexWrap="wrap" gap={0.5}>
                  {includedIngredients.map((ingredient, index) => (
                    <Chip 
                      key={index} 
                      label={ingredient} 
                      color="primary"
                      size="small"
                      onDelete={() => {
                        setIncludedIngredients(includedIngredients.filter((element) => element !== ingredient))
                      }}
                    />
                  ))}
                </Box>
              </Box>

              {/* Excluded Ingredients Section */}
              <Box display="flex" alignItems="center" gap={1}>
                <Typography variant="body2" sx={{ fontWeight: 'bold', minWidth: 'fit-content' }}>
                  Exclude:
                </Typography>
                <form 
                  onSubmit={(event) => {
                    event.preventDefault()
                    const formData = new FormData(event.currentTarget)
                    const newIngredient = formData.get('excludedIngredients')
                    if (newIngredient && newIngredient.trim()) {
                      setExcludedIngredients([...excludedIngredients, newIngredient.trim()])
                      event.target.reset()
                    }
                  }}
                >
                  <TextField
                    name="excludedIngredients"
                    variant="outlined"
                    placeholder="Add ingredient"
                    size="small"
                    sx={{
                      '& .MuiOutlinedInput-root': {
                        borderRadius: '15px',
                        bgcolor: 'white',
                        height: '32px'
                      },
                      width: '130px'
                    }}
                  />
                </form>
                <Box display="flex" flexWrap="wrap" gap={0.5}>
                  {excludedIngredients.map((ingredient, index) => (
                    <Chip 
                      key={index} 
                      label={ingredient} 
                      color="error"
                      size="small"
                      onDelete={() => {
                        setExcludedIngredients(excludedIngredients.filter((element) => element !== ingredient))
                      }}
                    />
                  ))}
                </Box>
              </Box>
            </Box>
          </Box>
        )}
      </AppBar>
      {/* {renderMobileMenu} */}
      {renderMenu}
    </Box>
  );
}
