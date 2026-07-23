import path from "node:path";

const config = {
  resolve: {
    alias: {
      "@": path.resolve(__dirname),
    },
  },
  test: {
    environment: "node",
  },
};

export default config;
