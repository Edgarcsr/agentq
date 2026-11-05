// Runs in the browser: signs checkout requests so the server can tell them apart.
export async function createCheckoutKey() {
  return crypto.subtle.generateKey(
    { name: 'ECDSA', namedCurve: 'P-256' },
    false,
    ['sign', 'verify'],
  );
}

export async function signCheckout(key, payload) {
  const data = new TextEncoder().encode(payload);
  return crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, key, data);
}
