import { cookies } from "next/headers";
import { redirect } from "next/navigation";

export default async function RootPage() {
  const store = await cookies();
  redirect(store.get("ame_token") ? "/library" : "/login");
}
