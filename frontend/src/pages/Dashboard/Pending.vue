<script setup lang="ts">
import {ref, onMounted, watch, computed, onBeforeUnmount} from "vue";
import LoadingCircle from "../../components/LoadingCircle.vue";
import ImageDiffer from "../../components/ImageDiffer.vue";
import type { PendingItem, PendingResponse } from "../../lib/types";
import { fetchJson } from "../../lib/utils";

const loading = ref(true);
const error = ref<string | null>(null);
const pendingItems = ref<PendingItem[]>([]);
const selectedItem = ref<PendingItem | null>(null);
const totalItems = ref(0);

const rejectReasonField = ref<HTMLInputElement | null>(null);
const rejectReason = ref<string>("");

const fullscreenLoading = ref(false);

const filterUsername = ref<string>("");
const filterLevelId = ref<string>("");
const filterReplacement = ref<string>("all"); // "all", "replacement", "new"

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const currentPage = ref(1);
const itemsPerPage = ref(12);
const pageInput = ref<string>("");

const filteredItems = computed(() => pendingItems.value);
const totalPages = computed(() => {
  return Math.ceil(totalItems.value / itemsPerPage.value);
});

const paginatedItems = computed(() => pendingItems.value);

watch([filterLevelId, filterUsername], () => {
  currentPage.value = 1;

  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }

  // debounce
  debounceTimer = setTimeout(() => {
    fetchPendingItems();
  }, 500);
});

watch(filterReplacement, () => {
  currentPage.value = 1;
  fetchPendingItems();
});

watch(itemsPerPage, () => {
  currentPage.value = 1;
  fetchPendingItems();
});

watch(currentPage, () => {
  fetchPendingItems();
});

watch(selectedItem, () => {
  rejectReason.value = "";
});

async function fetchPendingItems() {
  loading.value = true;
  error.value = null;

  try {
    const params = new URLSearchParams();
    params.append('page', currentPage.value.toString());
    params.append('per_page', itemsPerPage.value.toString());

    if (filterLevelId.value.trim() !== "") {
      params.append('level_id', filterLevelId.value.trim());
    }

    if (filterUsername.value.trim() !== "") {
      params.append('username', filterUsername.value.trim());
    }

    if (filterReplacement.value === "replacement") {
      params.append('replacement_only', 'true');
    } else if (filterReplacement.value === "new") {
      params.append('new_only', 'true');
    }

    const data = await fetchJson<PendingResponse>(`/pending?${params.toString()}`);

    pendingItems.value = data.uploads;
    totalItems.value = data.total;
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'An unknown error occurred';
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  fetchPendingItems();
});

function handlePopState(ev: PopStateEvent) {
  const state = ev.state as any;

  if (state && typeof state.pendingItemId === 'number') {
    const found = pendingItems.value.find(i => i.id === state.pendingItemId) || null;

    if (found) {
      selectedItem.value = found;
    } else {
      fetchPendingItems().then(() => {
        selectedItem.value = pendingItems.value.find(i => i.id === state.pendingItemId) || null;
      });
    }
  } else {
    selectedItem.value = null;
  }
}

onMounted(() => {
  window.addEventListener('popstate', handlePopState);
});

onBeforeUnmount(() => {
  window.removeEventListener('popstate', handlePopState);
});

async function thumbnailAction(id: number, accept: boolean) {
  if (!selectedItem.value || fullscreenLoading.value) return;
  if (!accept) {
    if (rejectReason.value.trim() === "") {
      rejectReasonField.value!.focus();
      return;
    }
  }

  fullscreenLoading.value = true;

  try {
    const response = await fetch(`/pending/${id}`, {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        accepted: accept,
        reason: accept ? null : rejectReason.value.trim()
      })
    });

    if (!response.ok) {
      const data = await response.json();
      throw new Error(data.message || 'Failed to process thumbnail action');
    }

    await fetchPendingItems();
    closeItem();
  } catch (err) {
    alert("An error occurred while processing the thumbnail action: " + (err instanceof Error ? err.message : 'Unknown error'));
  } finally {
    fullscreenLoading.value = false;
  }
}

function goToPage(page: number) {
  if (page >= 1 && page <= totalPages.value) {
    currentPage.value = page;
  }
}

function goToFirstPage() {
  currentPage.value = 1;
}

function goToLastPage() {
  currentPage.value = totalPages.value;
}

function handlePageInputChange() {
  const page = parseInt(pageInput.value, 10);
  if (!isNaN(page) && page >= 1 && page <= totalPages.value) {
    currentPage.value = page;
    pageInput.value = "";
  }
}

function openItem(item: PendingItem) {
  try {
    history.pushState({pendingItemId: item.id}, "", window.location.href);
  } catch (e) {
    // ignore
  }

  selectedItem.value = item;
}

function closeItem() {
  try {
    history.back();
  } catch (e) {
    selectedItem.value = null;
  }
}

</script>

<template>
  <div v-if="loading" class="d-flex flex-middle h-100">
    <LoadingCircle/>
  </div>
  <div v-else-if="error" class="error-message">
    <img src="/error.svg" alt="Error Icon" style="width: 128px; height: auto;"/>
    <p>{{ error }}</p>
  </div>
  <div v-else>
    <div v-if="selectedItem" class="selected-item page-transition">
      <button @click="closeItem()" class="btn btn-secondary">Back to Thumbnails</button>

      <h3 class="text-center">
        Level ID: {{ selectedItem!.level_id }}
        <span v-if="selectedItem!.replacement">(Replacement)</span><br/>
        Submitted by: {{ selectedItem!.username }}
      </h3>

      <ImageDiffer
          :src-a="`/pending/${selectedItem!.id}/image`"
          :src-b="selectedItem!.replacement ? `/thumbnail/${selectedItem!.level_id}` : undefined"
      />
      <div class="filler"></div>

      <div class="d-flex flex-col gap-1 w-100 sensitive-actions">
        <button @click="thumbnailAction(selectedItem!.id, true)" class="btn btn-success">
          Accept
        </button>
        <div class="d-flex">
          <input type="text" ref="rejectReasonField" v-model="rejectReason" required class="flex-3 form-control"
                 placeholder="Reason for rejection" list="rejectReasons"/>

          <datalist id="rejectReasons">
            <option value="Progress bar/percentage"/>
            <option value="Using Noclip"/>
            <option value="Low Quality"/>
            <option value="JPEGgy"/>
            <option value="Stretched"/>
            <option value="Title Card"/>
            <option value="Overlays"/>
          </datalist>

          <button @click="thumbnailAction(selectedItem!.id, false)" class="btn btn-danger flex-1">
            Reject
          </button>
        </div>
      </div>
    </div>
    <div v-else class="page-transition">
      <div class="filters-container">
        <div class="filters-grid">
          <div class="filter-item">
            <label for="filterReplacement">Type:</label>
            <select
                id="filterReplacement"
                v-model="filterReplacement"
                class="form-control"
            >
              <option value="all">All</option>
              <option value="replacement">Replacements Only</option>
              <option value="new">New Only</option>
            </select>
          </div>
          <div class="filter-item">
            <label for="itemsPerPage">Items per page:</label>
            <select
                id="itemsPerPage"
                v-model.number="itemsPerPage"
                class="form-control"
            >
              <option :value="12">12</option>
              <option :value="24">24</option>
              <option :value="36">36</option>
              <option :value="48">48</option>
              <option :value="60">60</option>
              <option :value="96">96</option>
            </select>
          </div>
          <div class="filter-item">
            <label for="filterUsername">Username:</label>
            <input
                id="filterUsername"
                type="text"
                v-model="filterUsername"
                class="form-control"
                placeholder="Search by username"
            />
          </div>
          <div class="filter-item">
            <label for="filterLevelId">Level ID:</label>
            <input
                id="filterLevelId"
                type="text"
                v-model="filterLevelId"
                class="form-control"
                placeholder="Search by level ID"
            />
          </div>
        </div>
        <div class="filter-results">
          Showing {{ pendingItems.length }} of {{ totalItems }} items
        </div>

        <div v-if="totalPages > 1" class="pagination-container">
          <div class="pagination-controls">
            <button
                @click="goToFirstPage()"
                :disabled="currentPage === 1"
                class="btn btn-secondary btn-sm"
                title="First page"
            >
              ⏮
            </button>

            <button
                @click="goToPage(currentPage - 1)"
                :disabled="currentPage === 1"
                class="btn btn-secondary btn-sm"
            >
              ◂
            </button>

            <span class="page-info">
              Page {{ currentPage }} of {{ totalPages }}
            </span>

            <button
                @click="goToPage(currentPage + 1)"
                :disabled="currentPage === totalPages"
                class="btn btn-secondary btn-sm"
            >
              ▸
            </button>

            <button
                @click="goToLastPage()"
                :disabled="currentPage === totalPages"
                class="btn btn-secondary btn-sm"
                title="Last page"
            >
              ⏭
            </button>

            <input
                type="number"
                v-model="pageInput"
                @keyup.enter="handlePageInputChange()"
                class="form-control pagination-input-field"
                placeholder="Go to page..."
                min="1"
                :max="totalPages"
            />
            <button
                @click="handlePageInputChange()"
                :disabled="pageInput === ''"
                class="btn btn-secondary btn-sm"
            >
              Go
            </button>
          </div>
        </div>
      </div>

      <div v-if="totalItems === 0" class="text-center mt-2">
        <p>No pending items found in the database.</p>
      </div>
      <div v-else-if="filteredItems.length === 0" class="text-center mt-2">
        <p>No items match the current filters.</p>
      </div>
      <div v-else class="image-grid">
        <div v-for="item in paginatedItems" :key="item.id" class="image-item" @click="openItem(item)">
          <img :src="`/pending/${item.id}/image`" alt="Thumbnail" class="thumbnail-image" loading="lazy"/>
          <div class="thumbnail-info">
            By {{ item.username }}<br/>
            Level ID: {{ item.level_id }}<br/>
            <span v-if="item.replacement" class="replacement-badge">Replacement</span>
          </div>
        </div>
      </div>
    </div>
  </div>
  <LoadingCircle backdrop v-if="fullscreenLoading"/>
</template>

<style scoped>
.error-message {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  font-size: 1.2em;
}

.filters-container {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 24px;
}

.filters-container h3 {
  margin-top: 0;
  margin-bottom: 16px;
}

.filters-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 12px;
}

.filter-item {
  display: flex;
  flex-direction: column;
}

.filter-item label {
  margin-bottom: 6px;
  font-weight: 500;
  font-size: 0.9em;
}

.filter-results {
  text-align: center;
  font-size: 0.95em;
  opacity: 0.8;
  margin-top: 8px;
}

.image-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 2fr));
  gap: 16px;
  margin: 24px 0;
}

.image-item {
  cursor: pointer;
  border-radius: 8px;
  overflow: hidden;
  transition: transform 0.2s;
  position: relative;
}

.image-item:hover {
  transform: scale(1.05);
}

.image-item:hover > .thumbnail-info {
  opacity: 1;
}

.thumbnail-info {
  padding: 8px;
  position: absolute;
  bottom: 0;
  color: #fff;
  background: linear-gradient(to top, rgba(0, 0, 0, 0.7), transparent);
  width: 100%;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.replacement-badge {
  display: inline-block;
  background: rgba(255, 165, 0, 0.8);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 0.85em;
  font-weight: bold;
  margin-top: 4px;
}

.pagination-container {
  display: flex;
  flex-direction: row;
  gap: 16px;
  align-items: center;
  justify-content: center;
  margin-top: 16px;
  padding: 8px;
}

.pagination-controls {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  justify-content: center;
}

.pagination-input-field {
  width: 120px;
  padding: 6px 12px;
  max-height: 32px;
}

.page-info {
  margin: 0 12px;
  font-weight: 500;
}

.btn-sm {
  padding: 6px 12px;
  font-size: 0.9em;
}

.thumbnail-image {
  width: 100%;
  height: auto;
  border-radius: 8px;
}

.selected-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-top: 20px;
  height: 100%;
}

.sensitive-actions {
  max-width: 800px;
}

.sensitive-actions > div {
  display: flex;
  justify-content: space-between;
}

@media (max-width: 768px) {
  .filters-grid {
    grid-template-columns: 1fr;
  }

  .pagination-controls {
    font-size: 0.9em;
    flex-wrap: wrap;
  }

  .page-info {
    margin: 0 8px;
  }

  .pagination-input-field {
    width: 100px;
  }
}
</style>