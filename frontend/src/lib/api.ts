export const API_BASE_URL: string =
  (import.meta.env.VITE_RELAY_HTTP_URL as string | undefined) ?? "http://localhost:3001";

const WS_BASE_URL: string = API_BASE_URL.replace(/^http/, "ws");

export interface DocSummary {
  id: string;
  title: string | null;
  created_at: string;
}

export async function createDoc(title?: string): Promise<DocSummary> {
  const res = await fetch(`${API_BASE_URL}/docs`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ title: title ?? null }),
  });
  if (!res.ok) {
    throw new Error(`failed to create document: HTTP ${res.status}`);
  }
  return (await res.json()) as DocSummary;
}

export async function listDocs(): Promise<DocSummary[]> {
  const res = await fetch(`${API_BASE_URL}/docs`);
  if (!res.ok) {
    throw new Error(`failed to list documents: HTTP ${res.status}`);
  }
  return (await res.json()) as DocSummary[];
}

export function wsUrlForDoc(docId: string): string {
  return `${WS_BASE_URL}/ws/docs/${docId}`;
}
