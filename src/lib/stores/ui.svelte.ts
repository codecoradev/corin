// UI state store — sidebar, panels, view mode
import { get } from 'svelte/store';

export function getUiStore() {
  let sidebarWidth = $state(280);
  let sidePanelVisible = $state(false);
  let graphViewVisible = $state(false);
  let detailPanelVisible = $state(true);
  let activeView: 'dashboard' | 'memories' | 'rooms' | 'graph' | 'settings' = $state('dashboard');
  let selectedMemoryId: string | null = $state(null);
  let dataDirInitialized = $state(false);
  let dataDir: string | null = $state(null);

  function toggleSidebar() { sidebarWidth = sidebarWidth > 0 ? 0 : 280; }
  function setGraphViewVisible(visible: boolean) { graphViewVisible = visible; }
  function setDetailPanelVisible(visible: boolean) { detailPanelVisible = visible; }
  function selectMemory(id: string | null) { selectedMemoryId = id; detailPanelVisible = true; }

  return {
    get sidebarWidth() { return sidebarWidth; },
    set sidebarWidth(v) { sidebarWidth = v; },
    get sidePanelVisible() { return sidePanelVisible; },
    set sidePanelVisible(v) { sidePanelVisible = v; },
    get graphViewVisible() { return graphViewVisible; },
    get detailPanelVisible() { return detailPanelVisible; },
    get activeView() { return activeView; },
    set activeView(v) { activeView = v; },
    get selectedMemoryId() { return selectedMemoryId; },
    get dataDirInitialized() { return dataDirInitialized; },
    set dataDirInitialized(v) { dataDirInitialized = v; },
    get dataDir() { return dataDir; },
    set dataDir(v) { dataDir = v; },
    toggleSidebar,
    setGraphViewVisible,
    setDetailPanelVisible,
    selectMemory,
  };
}
