"use client";

import { useRouter } from "next/navigation";
import {
  ArrowRight,
  ClipboardCheck,
  HeartPulse,
  History,
  Shield,
  Users,
} from "lucide-react";

const panels = [
  {
    title: "Task Review",
    description:
      "Review learner application tasks, provide feedback, and turn verified work into mastery evidence.",
    icon: ClipboardCheck,
    tone: "text-violet-500",
    link: "/admin/tasks",
    actionLabel: "Review Tasks",
  },
  {
    title: "Manage Users",
    description:
      "Search and filter platform users. Instantly manage roles, change subscription plans, or disable/enable accounts.",
    icon: Users,
    tone: "text-blue-500",
    link: "/admin/users",
    actionLabel: "Users Console",
  },
  {
    title: "Audit Logs",
    description:
      "Inspect the append-only system audit log. Filter by event action type, actor, or target identifier to trace operations.",
    icon: History,
    tone: "text-emerald-500",
    link: "/admin/audit",
    actionLabel: "Audit Trail",
  },
  {
    title: "System Health",
    description:
      "Check database and Valkey live statuses, view table row counts, and monitor rate-limit rejections.",
    icon: HeartPulse,
    tone: "text-red-500",
    link: "/admin/health",
    actionLabel: "System Status",
  },
];

export default function AdminDashboardPage() {
  const router = useRouter();
  return (
    <main className="px-6 py-12 sm:px-12">
      <header className="mb-12 flex items-center gap-4">
        <Shield size={46} className="text-primary" />
        <div>
          <h1 className="text-3xl font-bold">Admin Console</h1>
          <p className="mt-1 text-muted-foreground">
            Operational dashboard and system controls for the AME platform.
          </p>
        </div>
      </header>
      <div className="grid gap-6 sm:grid-cols-2 xl:grid-cols-4">
        {panels.map(
          ({ title, description, icon: Icon, tone, link, actionLabel }) => (
            <article
              key={title}
              className="flex h-full flex-col rounded-xl border border-border bg-card/80 p-7 shadow-sm backdrop-blur transition-all hover:-translate-y-1 hover:shadow-lg"
            >
              <div className="mb-5 inline-flex w-fit rounded-lg bg-muted/50 p-3">
                <Icon size={36} className={tone} />
              </div>
              <h2 className="text-xl font-semibold">{title}</h2>
              <p className="mt-3 flex-1 text-sm leading-6 text-muted-foreground">
                {description}
              </p>
              <button
                className="mt-7 flex w-full items-center justify-between rounded-md border border-border px-4 py-3 text-sm font-semibold transition-colors hover:border-primary hover:bg-primary hover:text-primary-foreground"
                onClick={() => router.push(link)}
              >
                {actionLabel}
                <ArrowRight size={17} />
              </button>
            </article>
          ),
        )}
      </div>
    </main>
  );
}
