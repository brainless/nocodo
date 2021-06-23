module.exports = {
  purge: ["./src/**/*.{js,jsx,ts,tsx}", "./public/index.html"],
  darkMode: false, // or 'media' or 'class'
  theme: {
    extend: {
      outline: {
        blue: "1px solid #0000ff",
      },
    },
  },
  variants: {
    extend: {
      outline: ["hover"],
    },
  },
  plugins: [],
};
