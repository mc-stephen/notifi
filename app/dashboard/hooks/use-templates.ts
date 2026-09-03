"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { Template, TemplateAttachment, TemplateContent } from "@/lib/types";

/// Partial input for creating a template.
export type CreateTemplateInput = {
  name: string;
  description?: string;
  channel: string;
  content?: TemplateContent;
  attachments?: TemplateAttachmentInput[];
};

/// Partial input for updating a template.
export type UpdateTemplateInput = {
  name: string;
  description?: string;
  channel: string;
  content?: TemplateContent;
  attachments?: TemplateAttachmentInput[];
};

export type TemplateAttachmentInput = {
  name: string;
  mimeType: string;
  sizeBytes: number;
  url: string;
};

type ListState = {
  templates: Template[];
  loading: boolean;
  error: string | null;
};

/// All templates for the current project, plus loading/error/refresh state.
export function useTemplates() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<ListState>({
    templates: [],
    loading: true,
    error: null,
  });

  useEffect(() => {
    if (!projectId) return;

    let ignore = false;
    (async () => {
      try {
        const res = await api<{ templates: Template[] }>(
          `/v1/projects/${projectId}/templates`,
        );
        if (ignore) return;
        setState({ templates: res.templates, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          templates: [],
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load templates",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setState((prev) => ({ ...prev, loading: true }));
    try {
      const res = await api<{ templates: Template[] }>(
        `/v1/projects/${projectId}/templates`,
      );
      setState({ templates: res.templates, loading: false, error: null });
    } catch (e) {
      setState({
        templates: [],
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load templates",
      });
    }
  }, [projectId]);

  return {
    templates: state.templates,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh,
  };
}

/// Create/update/delete actions scoped to the current project.
export function useTemplateActions() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const create = useCallback(
    async (input: CreateTemplateInput): Promise<Template> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ template: Template }>(
        `/v1/projects/${projectId}/templates`,
        { method: "POST", body: JSON.stringify(input) },
      );
      return res.template;
    },
    [projectId],
  );

  const update = useCallback(
    async (
      templateId: string,
      input: UpdateTemplateInput,
    ): Promise<Template> => {
      if (!projectId) throw new Error("No project selected");
      const res = await api<{ template: Template }>(
        `/v1/projects/${projectId}/templates/${templateId}`,
        { method: "PATCH", body: JSON.stringify(input) },
      );
      return res.template;
    },
    [projectId],
  );

  const remove = useCallback(
    async (templateId: string): Promise<void> => {
      if (!projectId) throw new Error("No project selected");
      await api(`/v1/projects/${projectId}/templates/${templateId}`, {
        method: "DELETE",
      });
    },
    [projectId],
  );

  return { create, update, remove };
}

type SingleState = {
  template: Template | null;
  loading: boolean;
  error: string | null;
};

/// A single template by id in the current project.
export function useTemplate(id: string) {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [state, setState] = useState<SingleState>({
    template: null,
    loading: true,
    error: null,
  });

  const load = useCallback(async () => {
    if (!projectId) return;
    try {
      const res = await api<{ template: Template }>(
        `/v1/projects/${projectId}/templates/${id}`,
      );
      setState({ template: res.template, loading: false, error: null });
    } catch (e) {
      setState({
        template: null,
        loading: false,
        error: e instanceof Error ? e.message : "Failed to load template",
      });
    }
  }, [id, projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) return;
      try {
        const res = await api<{ template: Template }>(
          `/v1/projects/${projectId}/templates/${id}`,
        );
        if (ignore) return;
        setState({ template: res.template, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState({
          template: null,
          loading: false,
          error: e instanceof Error ? e.message : "Failed to load template",
        });
      }
    })();
    return () => {
      ignore = true;
    };
  }, [id, projectId]);

  return {
    template: state.template,
    loading: state.loading && !!projectId,
    error: state.error,
    refresh: load,
  };
}

export type { TemplateAttachment };
