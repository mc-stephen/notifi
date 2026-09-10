// Renders a TOTP URI as a QR code without leaking the secret to a
// third-party image service. The `qrcode` package is loaded lazily so
// pages that don't need it stay light.

export async function renderQrCode(container: HTMLElement, uri: string): Promise<void> {
  const { default: QRCode } = (await import("qrcode")) as unknown as {
    default: { toDataURL(text: string, opts?: Record<string, unknown>): Promise<string> };
  };
  const dataUrl = await QRCode.toDataURL(uri, { width: 200, margin: 1 });
  container.innerHTML = "";
  const img = document.createElement("img");
  img.src = dataUrl;
  img.alt = "TOTP QR Code";
  img.width = 200;
  img.height = 200;
  img.className = "rounded";
  container.appendChild(img);
}
