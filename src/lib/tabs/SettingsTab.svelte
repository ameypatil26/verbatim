<script lang="ts">
  import type {
    AppSettings,
    HealthSnapshot,
    ModelsReport,
    RequiredModelId,
    ServiceRuntimeStatus
  } from '../types';
  import SettingsForm from '../components/SettingsForm.svelte';

  type ProgressState = {
    downloadedMb: number;
    totalMb: number;
    status: 'queued' | 'downloading' | 'done' | 'error';
    error?: string;
  };

  let {
    settings,
    health = null,
    models = null,
    serviceStatus = [],
    progress = {},
    downloading = false,
    onupdate,
    ondownload,
    ondelete,
    onrefresh,
    onrestart,
    onmodels
  }: {
    settings: AppSettings;
    health?: HealthSnapshot | null;
    models?: ModelsReport | null;
    serviceStatus?: ServiceRuntimeStatus[];
    progress?: Partial<Record<RequiredModelId, ProgressState>>;
    downloading?: boolean;
    onupdate?: (patch: Partial<AppSettings>) => void | Promise<void>;
    ondownload?: (model: RequiredModelId) => void | Promise<void>;
    ondelete?: (model: RequiredModelId) => void | Promise<void>;
    onrefresh?: () => void | Promise<void>;
    onrestart?: (name: 'asr' | 'ocr') => void | Promise<void>;
    onmodels?: () => void;
  } = $props();
</script>

<SettingsForm
  {settings}
  {health}
  {models}
  {serviceStatus}
  {progress}
  {downloading}
  {onupdate}
  {ondownload}
  {ondelete}
  {onrefresh}
  {onrestart}
  {onmodels}
/>
