export function canChangeCaptureConsent(state: {
  capture_supported: boolean;
  capture_opt_in: boolean;
}): boolean {
  // Unsupported packages cannot enable capture, but must still allow revocation.
  return state.capture_supported || state.capture_opt_in;
}
