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
import InboxIcon from '@mui/icons-material/MoveToInbox';
import MailIcon from '@mui/icons-material/Mail';
import { Create, CollectionsBookmark } from '@mui/icons-material';

export default function TemporaryDrawer(props) {
  const [open, setOpen] = React.useState(false);

  const router = useRouter()

  const toggleDrawer = (newOpen) => () => {
    setOpen(newOpen);
  };

  const drawerItemList = [
    {label: 'Create A Recipe', link: '/create', icon: <Create/>, divider: true},
    {label: 'My Recipes', link: '/', icon: <CollectionsBookmark/>}
  ]

  const DrawerList = (
    <Box sx={{ width: 250 }} role="presentation" onClick={props.setOpen}>
      <Box height={'56px'}></Box>  
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
