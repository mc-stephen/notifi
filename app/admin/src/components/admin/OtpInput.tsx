import { useEffect, useRef, useState } from "react";

const LENGTH = 6;

export default function OtpInput({
  onComplete,
  disabled = false,
}: {
  onComplete?: (code: string) => void;
  disabled?: boolean;
}) {
  const [digits, setDigits] = useState<string[]>(Array(LENGTH).fill(""));
  const inputs = useRef<Array<HTMLInputElement | null>>([]);

  // Put the cursor in the first box on arrival so typing starts immediately.
  useEffect(() => {
    inputs.current[0]?.focus();
  }, []);

  function focusAt(index: number) {
    inputs.current[index]?.focus();
    inputs.current[index]?.select();
  }

  function emit(next: string[]) {
    const code = next.join("");
    onComplete?.(code);
    window.dispatchEvent(new CustomEvent("otp:complete", { detail: code }));
  }

  function handleChange(index: number, value: string) {
    const clean = value.replace(/\D/g, "");
    if (!clean) return;
    const next = [...digits];
    // Spread pasted/typed chars across boxes from the current position.
    for (let i = 0; i < clean.length && index + i < LENGTH; i++) {
      next[index + i] = clean[i];
    }
    setDigits(next);
    const lastFilled = Math.min(index + clean.length, LENGTH) - 1;
    if (next.every((d) => d !== "")) {
      inputs.current[lastFilled]?.blur();
      emit(next);
    } else {
      focusAt(Math.min(lastFilled + 1, LENGTH - 1));
    }
  }

  function handleKeyDown(index: number, e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Backspace") {
      e.preventDefault();
      const next = [...digits];
      if (next[index] !== "") {
        next[index] = "";
        setDigits(next);
      } else if (index > 0) {
        next[index - 1] = "";
        setDigits(next);
        focusAt(index - 1);
      }
    } else if (e.key === "ArrowLeft" && index > 0) {
      focusAt(index - 1);
    } else if (e.key === "ArrowRight" && index < LENGTH - 1) {
      focusAt(index + 1);
    }
  }

  function handlePaste(e: React.ClipboardEvent<HTMLInputElement>) {
    e.preventDefault();
    const text = e.clipboardData.getData("text").replace(/\D/g, "").slice(0, LENGTH);
    if (!text) return;
    const next = Array(LENGTH).fill("");
    for (let i = 0; i < text.length; i++) next[i] = text[i];
    setDigits(next);
    if (text.length === LENGTH) {
      inputs.current[LENGTH - 1]?.blur();
      emit(next);
    } else {
      focusAt(text.length);
    }
  }

  return (
    <div className="flex items-center justify-center gap-3" role="group" aria-label="6-digit verification code">
      {digits.map((digit, i) => (
        <input
          key={i}
          ref={(el) => {
            inputs.current[i] = el;
          }}
          type="text"
          inputMode="numeric"
          autoComplete={i === 0 ? "one-time-code" : "off"}
          maxLength={1}
          value={digit}
          disabled={disabled}
          onChange={(e) => handleChange(i, e.target.value)}
          onKeyDown={(e) => handleKeyDown(i, e)}
          onPaste={handlePaste}
          onFocus={(e) => e.target.select()}
          aria-label={`Digit ${i + 1}`}
          className="size-12 rounded-lg border border-input bg-card text-center text-xl font-mono font-medium text-foreground transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50"
        />
      ))}
    </div>
  );
}
