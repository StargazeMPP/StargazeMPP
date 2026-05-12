/**
 * Stable error shape surfaced by the StargazeMPP gateway. Every
 * caller-facing fetch error wraps one of these.
 */
export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  readonly details?: unknown;

  constructor(code: string, message: string, status: number, details?: unknown) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
    this.details = details;
  }

  static async fromResponse(res: Response): Promise<ApiError> {
    let body: unknown;
    try {
      body = await res.json();
    } catch {
      body = undefined;
    }
    if (body && typeof body === "object" && "code" in body && "message" in body) {
      const b = body as { code: string; message: string; details?: unknown };
      return new ApiError(b.code, b.message, res.status, b.details);
    }
    return new ApiError("http_error", `Request failed with status ${res.status}`, res.status, body);
  }
}

export const ERROR_MESSAGES: Record<string, string> = {
  http_error: "Something went wrong on our end. Try again in a moment.",
  unauthorized: "Connect a wallet and sign the challenge to continue.",
  voucher_invalid: "Voucher rejected by the provider. Refresh and try again.",
  insufficient_escrow: "Session escrow exhausted. Top up to continue.",
  provider_unavailable: "Provider is offline. Your voucher was not charged.",
  rate_limited: "Too many requests. Wait a moment and try again.",
};

export function friendlyError(err: unknown): string {
  if (err instanceof ApiError) {
    return ERROR_MESSAGES[err.code] ?? err.message;
  }
  if (err instanceof Error) return err.message;
  return "Unknown error";
}
