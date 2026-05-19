let boundary = null;

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

export function enterIntentSession() {
  if (!boundary) {
    boundary = nowSeconds();
  }
  return boundary;
}

export function advanceIntentBoundary(timestamp) {
  boundary = timestamp;
  return boundary;
}

export function resetIntentSession() {
  boundary = null;
}
