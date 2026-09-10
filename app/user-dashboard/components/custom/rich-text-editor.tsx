"use client";

import { forwardRef, useCallback, useImperativeHandle, useState } from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Link from "@tiptap/extension-link";
import Image from "@tiptap/extension-image";
import Placeholder from "@tiptap/extension-placeholder";
import "prosemirror-view/style/prosemirror.css";
import { Bold, Italic, List, ListOrdered, Quote, Code, Link as LinkIcon, Image as ImageIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export type RichTextEditorHandle = {
  getHTML: () => string;
  isEmpty: () => boolean;
  clear: () => void;
};

function ToolbarBtn({
  active,
  onClick,
  children,
  title,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  title: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className={cn(
        "p-1.5 rounded-md transition-colors",
        active
          ? "bg-primary/10 text-primary"
          : "text-muted-foreground hover:text-foreground hover:bg-muted",
      )}
    >
      {children}
    </button>
  );
}

const RichTextEditor = forwardRef<
  RichTextEditorHandle,
  { placeholder?: string; onUpdate?: (html: string, empty: boolean) => void }
>(function RichTextEditor({ placeholder = "Write your message...", onUpdate }, ref) {
  const [isPreview, setIsPreview] = useState(false);

  const editor = useEditor(
    {
      extensions: [
        StarterKit,
        Link.configure({ openOnClick: false }),
        Image.configure({
          inline: false,
          allowBase64: true,
          HTMLAttributes: { class: "max-w-full h-auto rounded-md my-1" },
        }),
        Placeholder.configure({ placeholder }),
      ],
      immediatelyRender: false,
      editorProps: {
        attributes: {
          class:
            "min-h-[96px] max-h-[220px] overflow-y-auto px-3 py-2 text-xs focus:outline-none [&_p.is-editor-empty:first-child]:before:text-muted-foreground [&_p.is-editor-empty:first-child]:before:content-[attr(data-placeholder)] [&_p.is-editor-empty:first-child]:before:float-left [&_p.is-editor-empty:first-child]:before:pointer-events-none [&_p.is-editor-empty:first-child]:before:h-0",
        },
      },
      onUpdate: ({ editor }) => {
        onUpdate?.(editor.getHTML(), editor.isEmpty);
      },
    },
    [],
  );

  useImperativeHandle(
    ref,
    () => ({
      getHTML: () => editor?.getHTML() ?? "",
      isEmpty: () => editor?.isEmpty ?? true,
      clear: () => editor?.commands.clearContent(),
    }),
    [editor],
  );

  const setLink = useCallback(() => {
    if (!editor) return;
    const prev = editor.getAttributes("link").href;
    const url = window.prompt("Enter URL", prev);
    if (url === null) return;
    if (url === "") {
      editor.chain().focus().extendMarkRange("link").unsetLink().run();
      return;
    }
    editor.chain().focus().extendMarkRange("link").setLink({ href: url }).run();
  }, [editor]);

  const addImage = useCallback(() => {
    if (!editor) return;
    const url = window.prompt("Enter image URL");
    if (!url) return;
    editor.chain().focus().setImage({ src: url }).run();
  }, [editor]);

  if (!editor) return null;

  return (
    <div className="rounded-lg border border-input bg-transparent overflow-hidden">
      <div className="flex flex-wrap items-center gap-0.5 border-b border-border bg-muted/30 px-1.5 py-1">
        <ToolbarBtn
          active={editor.isActive("bold")}
          onClick={() => editor.chain().focus().toggleBold().run()}
          title="Bold"
        >
          <Bold className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          active={editor.isActive("italic")}
          onClick={() => editor.chain().focus().toggleItalic().run()}
          title="Italic"
        >
          <Italic className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          active={editor.isActive("bulletList")}
          onClick={() => editor.chain().focus().toggleBulletList().run()}
          title="Bullet List"
        >
          <List className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          active={editor.isActive("orderedList")}
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
          title="Numbered List"
        >
          <ListOrdered className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          active={editor.isActive("blockquote")}
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
          title="Quote"
        >
          <Quote className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn
          active={editor.isActive("codeBlock")}
          onClick={() => editor.chain().focus().toggleCodeBlock().run()}
          title="Code Block"
        >
          <Code className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn active={editor.isActive("link")} onClick={setLink} title="Link">
          <LinkIcon className="size-3.5" />
        </ToolbarBtn>
        <ToolbarBtn active={false} onClick={addImage} title="Insert Image">
          <ImageIcon className="size-3.5" />
        </ToolbarBtn>
        <div className="flex-1" />
        <button
          type="button"
          onClick={() => setIsPreview(!isPreview)}
          className="text-[11px] font-medium text-muted-foreground hover:text-foreground transition-colors px-1.5 py-1 rounded hover:bg-muted"
        >
          {isPreview ? "Edit" : "Preview"}
        </button>
      </div>

      {isPreview ? (
        <div
          className="min-h-[96px] max-h-[220px] overflow-y-auto px-3 py-2 text-xs"
          dangerouslySetInnerHTML={{
            __html: editor.getHTML() || '<span class="text-muted-foreground">Nothing to preview</span>',
          }}
        />
      ) : (
        <EditorContent editor={editor} />
      )}
    </div>
  );
});

export default RichTextEditor;
