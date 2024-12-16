'use client';
import { Roboto } from 'next/font/google';
import { Oswald } from 'next/font/google';
import { createTheme } from '@mui/material/styles';
import { darken } from '@mui/material/styles';

const roboto = Roboto({
  weight: ['300', '400', '500', '700'],
  subsets: ['latin'],
  display: 'swap',
});

const oswald = Oswald({ subsets: ['latin']})

const theme = createTheme({
  typography: {
    fontFamily: oswald.style.fontFamily,
  },
  palette: {
    primary: {
      main: 'rgb(239, 184, 56)', // Warm gold
    },
    secondary: {
        main: 'rgb(155, 175, 136)', // Soft sage green
    },
    error: {
        main: 'rgb(229, 104, 103)', // Muted coral
        contrastText: 'rgb(0,0,0)'
    },
    info: {
        main: 'rgb(96, 165, 191)', // Muted sky blue
    },
    warning: {
        main: 'rgb(248, 191, 132)', // Peach
    },
  },
  components: {
    MuiButton: {
        styleOverrides: {
            root: ({ownerState}) => ({
                margin: '8px',
                textTransform: 'none',
                fontWeight: 500,
                borderRadius: 8,
                transition: 'transform 0.2s ease-in-out',
                '&:hover': {
                    transform: 'scale(1.05)',
                    backgroundColor: darken(theme.palette[ownerState.color]?.main, 0.1),
                },
            }),
        },
    },
    MuiOutlinedInput: {
        styleOverrides: {
            root: {
                borderRadius: '8px',
                backgroundColor: 'rgba(240, 238, 225, 0.5)',
                '&:hover .MuiOutlinedInput-notchedOutline': {
                    borderColor: 'rgba(0, 0, 0, 0.5)',
                },
                '&.Mui-focused .MuiOutlinedInput-notchedOutline': {
                    borderColor: '#3f51b5',
                    borderWidth: '2px',
                },
                '&:hover': {
                    backgroundColor: 'rgba(240, m n 238, 225, 0.8)',
                },
                transition: 'all 0.3s ease',
            },
            input: {
                padding: '12px 14px',
            },
        },
    }
  },
});

export default theme;