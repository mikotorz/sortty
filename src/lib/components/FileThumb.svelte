<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { readFilePreview } from "../api/commands";
  import Image from "@lucide/svelte/icons/image";
  import Video from "@lucide/svelte/icons/video";
  import Music from "@lucide/svelte/icons/music";
  import FileText from "@lucide/svelte/icons/file-text";
  import FileCode from "@lucide/svelte/icons/file-code";
  import Package from "@lucide/svelte/icons/package";
  import Archive from "@lucide/svelte/icons/archive";
  import FileIcon from "@lucide/svelte/icons/file";

  let { path, size = 96 }: { path: string; size?: number } = $props();

  const IMAGE_EXT = new Set(["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"]);
  const VIDEO_EXT = new Set(["mp4", "mkv", "avi", "mov", "wmv", "webm"]);
  const AUDIO_EXT = new Set(["mp3", "wav", "flac", "aac", "ogg", "m4a"]);
  const DOC_EXT = new Set(["pdf", "doc", "docx", "txt", "md", "xls", "xlsx", "ppt", "pptx", "csv", "rtf"]);
  const CODE_EXT = new Set(["js", "ts", "py", "rs", "json", "html", "css", "svelte", "c", "cpp", "java", "go"]);
  const ARCHIVE_EXT = new Set(["zip", "rar", "7z", "tar", "gz"]);
  const INSTALLER_EXT = new Set(["exe", "msi", "msix"]);

  function extOf(p: string): string {
    const i = p.lastIndexOf(".");
    return i === -1 ? "" : p.slice(i + 1).toLowerCase();
  }

  function iconFor(ext: string) {
    if (VIDEO_EXT.has(ext)) return Video;
    if (AUDIO_EXT.has(ext)) return Music;
    if (DOC_EXT.has(ext)) return FileText;
    if (CODE_EXT.has(ext)) return FileCode;
    if (ARCHIVE_EXT.has(ext)) return Archive;
    if (INSTALLER_EXT.has(ext)) return Package;
    if (IMAGE_EXT.has(ext)) return Image;
    return FileIcon;
  }

  let ext = $derived(extOf(path));
  let isImage = $derived(IMAGE_EXT.has(ext));
  let Icon = $derived(iconFor(ext));

  let dataUrl = $state<string | null>(null);
  let failed = $state(false);
  let el = $state<HTMLElement>();
  let observer: IntersectionObserver | null = null;

  async function load() {
    if (!isImage || dataUrl || failed) return;
    try {
      dataUrl = await readFilePreview(path);
    } catch {
      failed = true;
    }
  }

  onMount(() => {
    if (!isImage || !el) return;
    observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          load();
          observer?.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(el);
  });

  onDestroy(() => observer?.disconnect());
</script>

<div
  bind:this={el}
  class="flex shrink-0 items-center justify-center overflow-hidden rounded-md bg-[var(--color-surface-hover)]"
  style="width: {size}px; height: {size}px;"
>
  {#if isImage && dataUrl}
    <img src={dataUrl} alt="" class="h-full w-full object-cover" />
  {:else}
    <Icon size={size * 0.4} class="text-[var(--color-text-muted)]" />
  {/if}
</div>
