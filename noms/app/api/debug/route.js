export default function handler(req, res) {
    res.status(200).json({ apiUrl: process.env.LAMBDA_API_URL });
}