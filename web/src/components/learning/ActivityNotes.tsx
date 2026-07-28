"use client";

import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { Button } from "@/components/ui/button";

type Note = {
  id: string;
  body: string;
  revision: number;
  contentVersion?: number | null;
};

export function ActivityNotes({
  journeyId,
  activityId,
  contentVersion,
}: {
  journeyId: string;
  activityId: string;
  contentVersion: number;
}) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [body, setBody] = useState("");
  const [editing, setEditing] = useState<Note | null>(null);
  const [saving, setSaving] = useState(false);

  async function reload() {
    const result = await api.GET("/api/v1/notes", {
      params: { query: { journeyId, activityId } },
    });
    if (result.response.ok && result.data) setNotes(result.data as Note[]);
  }

  useEffect(() => {
    void reload();
  }, [journeyId, activityId]);

  async function save() {
    if (!body.trim()) return;
    setSaving(true);
    const result = editing
      ? await api.PATCH("/api/v1/notes/{id}", {
          params: { path: { id: editing.id } },
          body: { expectedRevision: editing.revision, body },
        })
      : await api.POST("/api/v1/notes", {
          body: {
            journeyId,
            activityId,
            contentVersion,
            body,
            retryKey: crypto.randomUUID(),
          },
        });
    if (result.response.ok) {
      setBody("");
      setEditing(null);
      await reload();
    }
    setSaving(false);
  }

  async function remove(id: string) {
    const result = await api.DELETE("/api/v1/notes/{id}", {
      params: { path: { id } },
    });
    if (result.response.ok) await reload();
  }

  return (
    <aside
      className="mt-6 rounded-2xl border border-border bg-muted/30 p-5"
      data-testid="activity-notes"
    >
      <h3 className="font-semibold">Private notes</h3>
      <p className="mt-1 text-xs text-muted-foreground">
        Anchored to content version {contentVersion}. Only you can read these.
      </p>
      <textarea
        aria-label="Private note"
        className="mt-3 min-h-24 w-full rounded-xl border border-input bg-background p-3 text-sm outline-none focus:ring-2 focus:ring-ring"
        onChange={(event) => setBody(event.target.value)}
        placeholder="Capture what you want to remember…"
        value={body}
      />
      <div className="mt-2 flex gap-2">
        <Button disabled={saving || !body.trim()} onClick={() => void save()}>
          {editing ? "Save edit" : "Add note"}
        </Button>
        {editing && (
          <Button
            onClick={() => {
              setEditing(null);
              setBody("");
            }}
            variant="outline"
          >
            Cancel
          </Button>
        )}
      </div>
      <div className="mt-4 space-y-3">
        {notes.map((note) => (
          <article
            className="rounded-xl border border-border bg-card p-3"
            key={note.id}
          >
            <p className="whitespace-pre-wrap text-sm">{note.body}</p>
            <p className="mt-2 text-xs text-muted-foreground">
              Content version {note.contentVersion}
            </p>
            <div className="mt-2 flex gap-2">
              <Button
                onClick={() => {
                  setEditing(note);
                  setBody(note.body);
                }}
                size="sm"
                variant="outline"
              >
                Edit
              </Button>
              <Button
                onClick={() => void remove(note.id)}
                size="sm"
                variant="ghost"
              >
                Delete
              </Button>
            </div>
          </article>
        ))}
      </div>
    </aside>
  );
}
