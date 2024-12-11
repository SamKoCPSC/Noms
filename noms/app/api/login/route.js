import axios from "axios";

export default async function handler(req, res) {
    if (req.method !== 'POST') {
        return res.status(405).json({ error: 'Method not allowed' });
    }

    const {name, email} = req.body

    if(!email) {
        return res.status(400).json({error: 'Email is required'})
    }

    try {
        const response = await axios.post(process.env.LAMBDA_URL, {
          sql: `
            INSERT INTO users (name, email)
            VALUES ('${name}', '${email}')
            ON CONFLICT (email) DO NOTHING
          `,
        });
    
        res.status(200).json({ message: 'User created or already exists', data: response.data });
    } catch (error) {
    console.error(error);
    res.status(500).json({ error: 'Failed to create user' });
    }
}