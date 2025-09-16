'use client'
import * as React from 'react';
import axios from 'axios';
import { Box, Typography, Divider, TextField, Stack, Button } from "@mui/material"
import theme from "../theme"
import { useTheme } from "@emotion/react"
import { useFormik } from "formik"
import * as Yup from 'yup';
import { styled } from "@mui/material"
import { useRouter } from "next/navigation"
import { useSession } from "next-auth/react"
import { SnackBarContext } from "../layout";

const SubText = styled('p')( {
    fontSize: '11px',
    color: 'grey',
  })

export default function CreateCollection() {
    const theme = useTheme()
    const router = useRouter()
    const handleSnackBar = React.useContext(SnackBarContext)
    const {data: session, status} = useSession()
    const [isSubmitAttempted, setSubmitAttempted] = React.useState(false)

    const collectionFormik = useFormik({
        initialValues: {
            name: '',
            description: '',

        },
        initialErrors: {name: 'This just ensures that errors is not null so the error message is triggered'},
        validationSchema: Yup.object().shape({
            name: Yup.string().max(255, 'Name must be less than 255 characters').required('Name is required'),
            description: Yup.string().max(1000, 'Description must be less than 1000 characters').required('Description is required'),
        }),
        onSubmit: (values) => {
            axios.post(
                `/api/createCollection`,
                {
                    name: values.name,
                    description: values.description,
                },
                {
                    headers: {
                    'Content-Type': 'application/json',
                    }
                }
            ).then(() => {
                router.push('/')
                handleSnackBar('Collection has been successfully created!', theme.palette.success.main)
            }).catch(() => {
                handleSnackBar('Failed to create collection', theme.palette.error.main)
            })
        },
    })
    
    return (
        <Box display={"flex"} flexDirection={'column'}
        sx={{
          justifySelf: 'center',
          borderColor: 'rgb(230, 228, 215)',
          borderStyle: 'solid',
          borderWidth: 2,
          boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
          // gap: '20px', 
          backgroundColor: 'white',
          borderRadius: '30px',
          padding: '35px',
          marginY: '100px',
          [theme.breakpoints.up('870')]: {
            width: '800px'
          },
          [theme.breakpoints.down('870')]: {
            width: 'auto',
            marginX: '20px',
          },
        }}>
            <Typography sx={{
                [theme.breakpoints.up('476')]: {
                fontSize: '60px',
                },
                [theme.breakpoints.down('476')]: {
                fontSize: '40px',
                },
            }}>
                Create A Collection
            </Typography>
            <Divider sx={{marginY: '30px'}}></Divider>
            <Typography fontSize="30px">Name:</Typography>
            <SubText>Give your collection an identifiable name. The name will be used for searches and does not have to be unique.</SubText>
            <TextField 
            variant="outlined" 
            fullWidth
            name="name"
            id="name"
            value={collectionFormik.values.name}
            onChange={collectionFormik.handleChange}
            >
            </TextField>
            <Typography fontSize="30px">Description:</Typography>
            <SubText>Briefly describe your collection. Include any themes common to each recipe.</SubText>
            <TextField 
            variant="outlined" 
            fullWidth 
            multiline 
            name="description"
            id="description"
            value={collectionFormik.values.description}
            onChange={collectionFormik.handleChange}
            >
            </TextField>
            {/* <Divider sx={{marginY: '30px'}}></Divider>
            <Typography fontSize="30px">Recipes:</Typography>
            <SubText>Search and select recipes to include in the collection</SubText> */}
            <Stack direction={'row'} sx={{justifyContent: 'end'}}>
                <Button variant="contained" color="error" sx={{top: '30px'}} onClick={() => router.push('/')}>Cancel</Button>
                {/* {Object.keys(searchParams).length === 0 && <Button variant="contained" color="secondary" sx={{top: '30px'}} onClick={() => handleSave()}>Save As Draft</Button>} */}
                <Button disabled={status !== "authenticated" || (isSubmitAttempted && Object.keys(collectionFormik.errors).length > 0)} variant="contained" sx={{top: '30px'}} 
                onClick={() => {
                    setSubmitAttempted(true)
                    collectionFormik.handleSubmit()
                    if(Object.keys(collectionFormik.errors).length > 0) {handleSnackBar('Error: Please complete all required fields', theme.palette.error.main)}
                }}
                >Create And Publish</Button>
            </Stack>
        </Box>
    )
}