import { readPublicDoc } from "@/lib/publicDocs";

export const dynamic = "force-static";

export function GET() {
  return new Response(readPublicDoc("openapi.yaml"), {
    headers: { "content-type": "application/yaml; charset=utf-8" },
  });
}
