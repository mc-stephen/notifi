// Shared password policy — mirrors the backend `validate_password`
// (min 8 bytes + ASCII uppercase, lowercase, digit, punctuation).

export type PasswordCheck = {
  length: boolean;
  upper: boolean;
  lower: boolean;
  digit: boolean;
  special: boolean;
};

const ASCII_PUNCTUATION = /[!-/:-@\[-`{-~]/;

export function checkPassword(password: string): PasswordCheck {
  return {
    length: new TextEncoder().encode(password).length >= 8,
    upper: /[A-Z]/.test(password),
    lower: /[a-z]/.test(password),
    digit: /[0-9]/.test(password),
    special: ASCII_PUNCTUATION.test(password),
  };
}

export function passwordValid(password: string): boolean {
  const c = checkPassword(password);
  return c.length && c.upper && c.lower && c.digit && c.special;
}
