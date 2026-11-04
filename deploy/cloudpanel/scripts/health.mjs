const apiHost = process.env.MANIFESTADOR_NFE_API_HOST || '127.0.0.1';
const apiPort = process.env.MANIFESTADOR_NFE_API_PORT || '61001';
const webHost = process.env.MANIFESTADOR_NFE_WEB_HOST || '127.0.0.1';
const webPort = process.env.PORT || process.env.MANIFESTADOR_NFE_WEB_PORT || '61002';

async function check(url) {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(5000) });
    const body = await response.text();
    console.log(`[OK] ${url}`);
    if (body) console.log(body.slice(0, 500));
    return response.ok;
  } catch (error) {
    console.error(`[FAIL] ${url}: ${error.message}`);
    return false;
  }
}

const apiOk = await check(`http://${apiHost}:${apiPort}/health`);
const webOk = await check(`http://${webHost}:${webPort}/`);
process.exit(apiOk && webOk ? 0 : 1);
