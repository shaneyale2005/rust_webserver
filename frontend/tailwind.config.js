export default {
    content: [
        "./index.html",
        "./src/**/*.{vue,js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {
            colors: {
                'cream': '#FAF7F0',
                'sand': '#E8DCC4',
                'tan': '#D4C5A9',
                'brown': '#9C8671',
                'dark-brown': '#6B5D52',
                'darker-brown': '#4A3F35',
            },
            fontFamily: {
                'sans': ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', 'sans-serif'],
            },
        },
    },
    plugins: [],
}
