import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";
import AccessDenied from "@/app/components/AccessDenied";
import { Container, Typography, Box, Table, TableBody, TableCell, TableContainer, TableRow, Paper } from "@mui/material";
import Link from "next/link";

async function getUserDrafts(userId) {
  return fetch(process.env.LAMBDA_API_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": process.env.LAMBDA_API_KEY,
    },
    body: JSON.stringify({
      sql: `
        SELECT url, createdat
        FROM recipe_drafts
        WHERE userid = %s
        ORDER BY createdat DESC
      `,
      values: [userId],
    }),
  })
    .then((response) => {
      if (!response.ok) {
        throw new Error(`HTTP error! Status: ${response.status}`);
      }
      return response.json();
    })
    .then((data) => {
      return data.result || [];
    })
    .catch((error) => {
      console.error(error);
      return [];
    });
}

function getParameterFromUrl(url, paramName) {
  try {
    const queryString = url.split('?')[1];
    if (!queryString) return '';
    const params = new URLSearchParams(queryString);
    return params.get(paramName) || '';
  } catch {
    return '';
  }
}

function formatTimestamp(timestamp) {
  if (!timestamp) return 'Unknown date';
  const isoTimestamp = timestamp.replace(" ", "T");
  const date = new Date(isoTimestamp);
  if (isNaN(date.getTime())) {
    return 'Unknown date';
  }
  const options = {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  };
  return date.toLocaleDateString(undefined, options);
}

export default async function DraftsPage() {
  const session = await getServerSession(authOptions);
  if (!session) {
    return <AccessDenied />;
  }

  const drafts = await getUserDrafts(session.user.id);

  const textStyle = {
    titleSize: '4.5rem',
    sectionTitleSize: '3.125rem',
    listItemSize: '2rem',
    paragraphSize: '1.25rem'
  }

  return (
    <Container maxWidth='false' sx={{ justifyItems: 'center' }}>
      <Box 
        display="flex"
        alignItems="flex-start"
        sx={{
          width: '100%',
          backgroundColor: 'white',
          padding: '20px',
          margin: '30px',
          borderRadius: '15px',
          borderColor: 'rgb(230, 228, 215)',
          borderStyle: 'solid',
          borderWidth: 2,
          boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
        }}
      >
        <Box sx={{ flex: 1 }}>
          <Typography
            sx={{ 
              fontSize: textStyle.titleSize,
              marginBottom: '0px',
              textAlign: 'left'
            }}
          >
            My Drafts
          </Typography>
        </Box>
        <Box 
          display="flex" 
          flexDirection="row" 
          alignItems="flex-end"
          sx={{ gap: '15px' }}
        >
          <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
            <Typography 
              variant="h4" 
              sx={{ 
                fontSize: textStyle.sectionTitleSize,
                fontWeight: 'bold',
                color: 'secondary.main'
              }}
            >
              {drafts.length || 0}
            </Typography>
            <Typography 
              variant="body2" 
              sx={{ 
                fontSize: textStyle.paragraphSize,
                color: 'text.secondary'
              }}
            >
              {drafts.length === 1 ? 'Draft' : 'Drafts'}
            </Typography>
          </Box>
        </Box>
      </Box>

      {drafts.length === 0 ? (
        <Box 
          sx={{
            width: '100%',
            backgroundColor: 'white',
            padding: '40px',
            margin: '30px',
            borderRadius: '15px',
            borderColor: 'rgb(230, 228, 215)',
            borderStyle: 'solid',
            borderWidth: 2,
            boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
            textAlign: 'center'
          }}
        >
          <Typography sx={{ fontSize: textStyle.paragraphSize, color: 'text.secondary' }}>
            No drafts found. Start creating a new recipe draft!
          </Typography>
        </Box>
      ) : (
        <Box 
          display="flex"
          flexDirection={'column'}
          alignItems="flex-start"
          sx={{
            width: '100%',
            backgroundColor: 'white',
            paddingTop: '20px',
            paddingBottom: '5px',
            margin: '30px',
            borderRadius: '15px',
            borderColor: 'rgb(230, 228, 215)',
            borderStyle: 'solid',
            borderWidth: 2,
            boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
          }}
        >
          <Typography
            sx={{ 
              fontSize: '1.7rem',
              textAlign: 'left',
              lineHeight: 1.5,
              marginLeft: '20px',
              marginBottom: '10px',
            }}
          >
            Drafts
          </Typography>
          <TableContainer 
            component={Paper}
            sx={{
              width: '100%',
              borderRadius: 0,
              border: 'none',
              boxShadow: 'none'
            }}
          >
            <Table sx={{ width: '100%' }}>
              <TableBody sx={{ borderTop: '1px solid black' }}>
              {drafts.map((draft, index) => {
                const draftName = getParameterFromUrl(draft.url, 'name') || 'Untitled Draft';
                const draftDescription = getParameterFromUrl(draft.url, 'description') || '';
                return (
                  <Link href={draft.url} key={index} style={{ textDecoration: 'none', display: 'contents' }}>
                    <TableRow 
                      sx={{ 
                        '&:hover': { backgroundColor: 'rgba(0, 0, 0, 0.04)', cursor: 'pointer' },
                        borderTop: index === 0 ? 'none' : '1px solid black',
                      }}
                    >
                      <TableCell sx={{ paddingY: '16px', width: '100%', borderBottom: 'none' }}>
                        <Typography sx={{ fontSize: '1.1rem', fontWeight: 500 }}>
                          {draftName}
                        </Typography>
                        <Typography sx={{ fontSize: '0.85rem', color: 'text.secondary', marginTop: '4px' }}>
                          Saved: {formatTimestamp(draft.createdat)}
                        </Typography>
                        <Typography sx={{ fontSize: '0.9rem', color: 'text.secondary', marginTop: '8px', lineHeight: 1.4, fontStyle: draftDescription ? 'normal' : 'italic' }}>
                          {draftDescription || 'no description'}
                        </Typography>
                      </TableCell>
                    </TableRow>
                  </Link>
                );
              })}
            </TableBody>
          </Table>
          </TableContainer>
        </Box>
      )}
    </Container>
  );
}