import { loadConfig } from "./config.js";
import { createApp } from "./app.js";
const config = loadConfig();
const app = await createApp(config);
app.listen(config.port, config.host, () => console.log(JSON.stringify({ event: "gateway_listen", host: config.host, port: config.port, mode: config.mode })));
