import { readPublicDoc } from "@/lib/publicDocs";

export const dynamic = "force-static";

export function GET() {
  return new Response(readPublicDoc("skill.json"), {
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}
