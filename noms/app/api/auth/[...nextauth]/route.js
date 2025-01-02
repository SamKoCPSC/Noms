import NextAuth from "next-auth/next";
import GoogleProvider from "next-auth/providers/google"
import axios from 'axios'

export const authOptions = {
    providers: [
        GoogleProvider({
            clientId: process.env.GOOGLE_CLIENT_ID,
            clientSecret: process.env.GOOGLE_CLIENT_SECRET,
            authorization: {
                params: {
                  prompt: "consent",
                  access_type: "offline",
                  response_type: "code"
                }
              }
        }),
    ],
    secret: process.env.SECRET,
    callbacks: {
      async signIn({ user }) {
        try {
          await axios.post(
            `${process.env.NOMS_URL}/api/login`,
            {
              name: user.name,
              email: user.email,
            },
            {
              headers: {
                'Content-Type': 'application/json',
            }
          })
          return true
        } catch (error) {
          console.error('Failed to create user:', error)
          return false
        }
      },
      async session({ session, token, user }) {
        // Use given_name and family_name if you want structured data
        session.user.firstName = token.given_name || session.user.name?.split(" ")[0];
        session.user.lastName = token.family_name || session.user.name?.split(" ")[1] || "";
        return session;
      },
    },
}

const handler = NextAuth(authOptions)
export { handler as GET, handler as POST }