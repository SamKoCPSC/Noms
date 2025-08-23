'use client'
import * as React from 'react';
import { useRouter } from 'next/navigation';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import Button from '@mui/material/Button';
import List from '@mui/material/List';
import Divider from '@mui/material/Divider';
import ListItem from '@mui/material/ListItem';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import { Login, Logout } from '@mui/icons-material';
import { Create, CollectionsBookmark, AccountBox, Info } from '@mui/icons-material';
import { useSession } from 'next-auth/react';
import { signIn, signOut } from 'next-auth/react';

export default function TemporaryDrawer(props) {
  const {data: session, status} = useSession()
  const [open, setOpen] = React.useState(false);

  const router = useRouter()
  
  // Use the dynamic navbar height passed from parent
  const spacerHeight = `${props.navbarHeight || 56}px`;

  const toggleDrawer = (newOpen) => () => {
    setOpen(newOpen);
  };

  const drawerItemList = [
    {label: 'Create A Recipe', link: status === 'authenticated' ? `/createRecipe` : '/signInRequired', icon: <Create/>, divider: true},
    {label: 'My Recipes', link: status === 'authenticated' ? `/myRecipes/${session.user.id}` : '/signInRequired', icon: <CollectionsBookmark/>},
    {label: 'Account', link: status === 'authenticated' ? `/account/${session.user.id}` : '/signInRequired', icon: <AccountBox/>, divider: true},
    {label: 'About', link: '/about', icon: <Info/>},
  ]

  const DrawerList = (
    <Box sx={{ width: 250 }} role="presentation" onClick={props.setOpen}>
      <Box height={spacerHeight}></Box>  
      <List>
        {drawerItemList.map((drawerItem) => (
          <Box key={drawerItem.label}>
            <ListItem disablePadding>
              <ListItemButton onClick={() => router.push(`${drawerItem.link}`)}>
                <ListItemIcon>
                  {drawerItem.icon}
                </ListItemIcon>
                <ListItemText primary={drawerItem.label} />
              </ListItemButton>
            </ListItem>
            {drawerItem.divider && <Divider/>}
          </Box>
        ))}
          <Divider/>
          <Box>
            <ListItem disablePadding>
              <ListItemButton onClick={() => status === 'authenticated' ? signOut({callbackUrl: '/'}) : signIn('google') }>
                <ListItemIcon>
                  {status === 'authenticated' ? <Logout/> : <Login/> }
                </ListItemIcon>
                <ListItemText primary={status === 'authenticated' ? 'Logout' : 'Login'} />
              </ListItemButton>
            </ListItem>
          </Box>
          <Divider/>
      </List>
    </Box>
  );

  return (
    <Box>
      {/* <Button onClick={toggleDrawer(true)}>Open drawer</Button> */}
      <Drawer open={props.open} onClose={props.setOpen}>
        {DrawerList}
      </Drawer>
    </Box>
  );
}
