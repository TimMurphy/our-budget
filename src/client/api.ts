/** Send a command or query to the API (routed to the server-side mediator). */
export async function send<T>(type: string, body: object = {}): Promise<T> {
  const res = await fetch(`/api/${type}`, { method: "POST", body: JSON.stringify(body) });
  if (!res.ok) throw new Error((await res.json()).error);
  return res.json();
}
