// OAuth2 utility functions

// Generate random string
function generateRandomString(length: number): string {
  const charset =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
  let result = "";
  const randomValues = new Uint8Array(length);
  crypto.getRandomValues(randomValues);

  for (let i = 0; i < length; i++) {
    result += charset[randomValues[i] % charset.length];
  }

  return result;
}

// Generate code verifier
export function generateCodeVerifier(): string {
  return generateRandomString(64);
}

// Generate code challenge (SHA-256)
export async function generateCodeChallenge(verifier: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(verifier);
  const hash = await crypto.subtle.digest("SHA-256", data);

  // Convert to base64url
  return btoa(String.fromCharCode(...new Uint8Array(hash)))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=/g, "");
}

// Generate state
export function generateState(): string {
  return generateRandomString(32);
}

// Build authorization URL
export function buildAuthorizeUrl(
  authorizeUrl: string,
  clientId: string,
  redirectUri: string,
  state: string,
  codeChallenge: string,
  scope: string
): string {
  const params = new URLSearchParams({
    response_type: "code",
    client_id: clientId,
    redirect_uri: redirectUri,
    scope: scope,
    state: state,
    code_challenge: codeChallenge,
    code_challenge_method: "S256",
  });

  return `${authorizeUrl}?${params.toString()}`;
}

// Store OAuth2 session
export function storeOAuthSession(state: string, codeVerifier: string): void {
  sessionStorage.setItem("oauth_state", state);
  sessionStorage.setItem("oauth_code_verifier", codeVerifier);
}

// Get and clear OAuth2 session
export function getAndClearOAuthSession(): {
  state: string;
  codeVerifier: string;
} | null {
  const state = sessionStorage.getItem("oauth_state");
  const codeVerifier = sessionStorage.getItem("oauth_code_verifier");

  if (state && codeVerifier) {
    sessionStorage.removeItem("oauth_state");
    sessionStorage.removeItem("oauth_code_verifier");
    return { state, codeVerifier };
  }

  return null;
}
