'use client'
import * as React from 'react';
import axios from 'axios';
import { Box, Typography, Divider, TextField, Stack, Button, IconButton, InputAdornment } from "@mui/material"
import { Search } from '@mui/icons-material';
import theme from "../theme"
import { useTheme } from "@emotion/react"
import { useFormik } from "formik"
import * as Yup from 'yup';
import { styled } from "@mui/material"
import { useRouter } from "next/navigation"
import { useSession } from "next-auth/react"
import { SnackBarContext } from "../layout";
import RecipeCardMini from '../components/RecipeCardMini';

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
    const [searchResults, setSearchResults] = React.useState([])
    const [selectedVariants, setSelectedVariants] = React.useState([])

    const handleAddVariant = (variant) => {
        setSelectedVariants([...selectedVariants, variant])
    }

    const handleRemoveVariant = (variant) => {
        setSelectedVariants(selectedVariants.filter(selectedVariant => selectedVariant.branchid !== variant.branchid))
    }

    const collectionFormik = useFormik({
        initialValues: {
            name: '',
            description: '',
            variantIds: []

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
                    variantIds: values.variantIds
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

    const handleSearch = (event) => {
        event.preventDefault()
        const formData = new FormData(event.currentTarget)
        fetch(
        `/api/searchVariants?variantName=${formData.get('variantSearch')}`
        ).then((response) => {
            if(!response.ok) {
                console.error(response)
                throw new Error(`HTTP error! Status: ${response.status}`)
            }
            return response.json()
        }).then((data) => {
            setSearchResults(data.result)
        })
        .catch((error) => {
            console.error(error)
            return {message: 'error'}
        })
    }
    
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
            <Divider sx={{marginY: '30px'}}></Divider>
            <Typography fontSize="30px">Recipes:</Typography>
            <SubText>Add recipe variants to the collection</SubText>
            <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} 
                sx={{
                    gap: '20px',
                    maxHeight: '150px',
                    overflowY: 'auto',
                    padding: '5px',
                    [theme.breakpoints.down('645')]: {justifyContent: 'center'}
                }}>
                {selectedVariants.map((result, index) => {
                    return <RecipeCardMini key={index} id={result.branchid} name={result.latestrecipename} variantName={result.variantname} ownerName={result.ownername} imageURLs={result.latestimageurls} handleClick={handleRemoveVariant}/>
                })}
            </Box>
            <form onSubmit={handleSearch} style={{flexGrow: 1}}>
                <TextField
                name="variantSearch"
                variant="outlined"
                placeholder="Search for recipes"
                InputProps={{
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
            <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} 
                sx={{
                    gap: '20px',
                    marginTop: '10px',
                    maxHeight: '450px',
                    overflowY: 'auto',
                    padding: '5px',
                    [theme.breakpoints.down('645')]: {justifyContent: 'center'}
                }}>
                {searchResults.map((result, index) => {
                    return <RecipeCardMini key={index} id={result.branchid} name={result.latestrecipename} variantName={result.variantname} ownerName={result.ownername} imageURLs={result.latestimageurls} handleClick={handleAddVariant}/>
                })}
            </Box>
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