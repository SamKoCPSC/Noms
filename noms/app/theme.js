'use client';
import { Roboto } from 'next/font/google';
import { Oswald } from 'next/font/google';
import { createTheme } from '@mui/material/styles';

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
});

export default theme;