import { readPublicDoc } from "@/lib/publicDocs";

export const dynamic = "force-static";

export function GET() {
  return new Response(readPublicDoc("llms.txt"), {
    headers: { "content-type": "text/plain; charset=utf-8" },
  });
}
