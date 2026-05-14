<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import LoadingCircle from '../../components/LoadingCircle.vue';
import { fetchJson, unwrap } from '../../lib/utils';
import type { UserListResponse, UserRow } from '../../lib/types';

type SortColumn = 'id' | 'username' | 'role' | 'total_uploads' | 'accepted' | 'pending' | 'rejected' | 'active_thumbnails';
type SortDirection = 'asc' | 'desc';

const loading = ref(true);
const refreshing = ref(false);
const error = ref<string | null>(null);

const users = ref<UserRow[]>([]);
const total = ref(0);

const currentPage = ref(1);
const itemsPerPage = ref(10);
const pageInput = ref('');
const itemsPerPageOptions = [10, 25, 50, 100] as const;

const filterUsername = ref('');
const filterId = ref('');
const filterAccountId = ref('');
const filterDiscordId = ref('');
const filterRole = ref('');

const sortBy = ref<SortColumn>('id');
const sortDirection = ref<SortDirection>('asc');

const copiedKey = ref<string | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const totalPages = computed(() =>
  total.value === 0 ? 0 : Math.ceil(total.value / itemsPerPage.value)
);

function buildParams(): URLSearchParams {
  const p = new URLSearchParams({
    page: String(currentPage.value),
    per_page: String(itemsPerPage.value),
    sort_by: sortBy.value,
    sort_dir: sortDirection.value,
  });
  if (filterUsername.value.trim()) p.append('username', filterUsername.value.trim());
  if (filterId.value.trim()) p.append('id', filterId.value.trim());
  if (filterAccountId.value.trim()) p.append('account_id', filterAccountId.value.trim());
  if (filterDiscordId.value.trim()) p.append('discord_id', filterDiscordId.value.trim());
  if (filterRole.value) p.append('role', filterRole.value);
  return p;
}

async function fetchUsers() {
  const initial = loading.value && users.value.length === 0;
  if (!initial) refreshing.value = true;
  error.value = null;

  try {
    const payload = await fetchJson<unknown>(`/admin/users?${buildParams()}`);
    const data = unwrap<UserListResponse>(payload);
    users.value = data.users;
    total.value = data.total;
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'An unknown error occurred';
  } finally {
    if (initial) loading.value = false;
    refreshing.value = false;
  }
}

function scheduleFetch() {
  currentPage.value = 1;
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(fetchUsers, 400);
}

watch([filterUsername, filterId, filterAccountId, filterDiscordId, filterRole], scheduleFetch);
watch(itemsPerPage, () => { currentPage.value = 1; fetchUsers(); });
watch(currentPage, fetchUsers);

onMounted(fetchUsers);
onBeforeUnmount(() => {
  if (copiedTimer) clearTimeout(copiedTimer);
  if (debounceTimer) clearTimeout(debounceTimer);
});

function toggleSort(col: SortColumn) {
  if (sortBy.value === col) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc';
  } else {
    sortBy.value = col;
    sortDirection.value = (col === 'id' || col === 'username') ? 'asc' : 'desc';
  }
  fetchUsers();
}

function sortIcon(col: SortColumn) {
  if (sortBy.value !== col) return '/icons/ascending.svg';
  return sortDirection.value === 'asc' ? '/icons/ascending.svg' : '/icons/descending.svg';
}

function roleIcon(role: UserRow['role']): string {
  const map: Record<UserRow['role'], string> = {
    user: '/icons/user.svg',
    verified: '/icons/verified-user.svg',
    moderator: '/icons/moderator.svg',
    admin: '/icons/admin.svg',
  };
  return map[role];
}

async function copyToClipboard(text: string, key: string) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const el = Object.assign(document.createElement('textarea'), {
        value: text,
        readOnly: true,
      });
      Object.assign(el.style, { position: 'fixed', opacity: '0' });
      document.body.appendChild(el);
      el.select();
      document.execCommand('copy');
      document.body.removeChild(el);
    }
    copiedKey.value = key;
    if (copiedTimer) clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => {
      if (copiedKey.value === key) copiedKey.value = null;
    }, 1200);
  } catch { /* ignore */ }
}

function goToPage(page: number) {
  if (page >= 1 && page <= totalPages.value) currentPage.value = page;
}

function handlePageInput() {
  const page = parseInt(pageInput.value, 10);
  if (!Number.isNaN(page) && page >= 1 && page <= totalPages.value) {
    currentPage.value = page;
    pageInput.value = '';
  }
}
</script>

<template>
  <div v-if="loading" class="centered-fill">
    <LoadingCircle />
  </div>

  <div v-else class="users-page">
    <div v-if="error && users.length === 0" class="error-state">
      <img src="/error.svg" alt="Error" class="error-img" />
      <p>{{ error }}</p>
    </div>

    <template v-else>
      <section class="panel">
        <header class="panel-header">
          <span class="panel-kicker">Users</span>
          <span class="panel-count">Total {{ total }}</span>
        </header>

        <div class="filters-grid">
          <label class="filter-label">
            Username
            <input v-model="filterUsername" class="ctrl" type="text" placeholder="Search by username" />
          </label>
          <label class="filter-label">
            User ID
            <input v-model="filterId" class="ctrl" type="text" inputmode="numeric" placeholder="Search by user ID" />
          </label>
          <label class="filter-label">
            GD Account ID
            <input v-model="filterAccountId" class="ctrl" type="text" inputmode="numeric"
              placeholder="Search by GD account ID" />
          </label>
          <label class="filter-label">
            Discord ID
            <input v-model="filterDiscordId" class="ctrl" type="text" inputmode="numeric"
              placeholder="Search by Discord ID" />
          </label>
          <label class="filter-label">
            Role
            <select v-model="filterRole" class="ctrl select">
              <option value="">Any</option>
              <option value="user">user</option>
              <option value="verified">verified</option>
              <option value="moderator">moderator</option>
              <option value="admin">admin</option>
            </select>
          </label>
          <label class="filter-label">
            Per page
            <select v-model.number="itemsPerPage" class="ctrl select">
              <option v-for="n in itemsPerPageOptions" :key="n" :value="n">{{ n }}</option>
            </select>
          </label>
        </div>

        <nav v-if="totalPages > 1" class="pagination">
          <button class="btn btn-secondary btn-sm" title="First page" :disabled="currentPage === 1"
            @click="goToPage(1)">
            <img src="/icons/first.svg" alt="⏮" class="nav-icon" />
          </button>
          <button class="btn btn-secondary btn-sm" title="Previous page" :disabled="currentPage === 1"
            @click="goToPage(currentPage - 1)">
            <img src="/icons/previous.svg" alt="◂" class="nav-icon" />
          </button>

          <span class="page-info">Page {{ currentPage }} of {{ totalPages }}</span>

          <button class="btn btn-secondary btn-sm" title="Next page" :disabled="currentPage === totalPages"
            @click="goToPage(currentPage + 1)">
            <img src="/icons/next.svg" alt="▸" class="nav-icon" />
          </button>
          <button class="btn btn-secondary btn-sm" title="Last page" :disabled="currentPage === totalPages"
            @click="goToPage(totalPages)">
            <img src="/icons/last.svg" alt="⏭" class="nav-icon" />
          </button>

          <input v-model="pageInput" class="ctrl page-input" type="number" placeholder="Go to…" min="1"
            :max="totalPages" @keyup.enter="handlePageInput" />
          <button class="btn btn-secondary btn-sm" :disabled="!pageInput" @click="handlePageInput">Go</button>
        </nav>
      </section>

      <div class="table-wrap">
        <table class="users-table">
          <thead>
            <tr>
              <th v-for="col in ([
                { key: 'id', label: 'ID' },
                { key: 'username', label: 'User' },
                { key: 'role', label: 'Role' },
                { key: 'total_uploads', label: 'Uploads' },
                { key: 'accepted', label: 'Accepted' },
                { key: 'pending', label: 'Pending' },
                { key: 'rejected', label: 'Rejected' },
                { key: 'active_thumbnails', label: 'Active' },
              ] as const)" :key="col.key">
                <button class="sort-btn" @click="toggleSort(col.key)">
                  {{ col.label }}
                  <img :src="sortIcon(col.key)" :class="['sort-icon', { active: sortBy === col.key }]" alt="" />
                </button>
              </th>
              <th class="actions-th">Actions</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="u in users" :key="u.id">
              <td><span class="mono">{{ u.id }}</span></td>

              <td>
                <div class="user-cell">
                  <span class="username">{{ u.username }}</span>
                  <div class="meta-icons">
                    <button v-if="u.account_id !== -1" class="icon-btn" :title="`GD account ID: ${u.account_id}`"
                      @click="copyToClipboard(String(u.account_id), `account:${u.id}`)">
                      <img :src="copiedKey === `account:${u.id}` ? '/icons/copied.svg' : '/icons/dash.svg'" alt="GD"
                        class="icon-sm" />
                    </button>
                    <button v-if="u.discord_id" class="icon-btn" :title="`Discord ID: ${u.discord_id}`"
                      @click="copyToClipboard(u.discord_id, `discord:${u.id}`)">
                      <img :src="copiedKey === `discord:${u.id}` ? '/icons/copied.svg' : '/icons/discord.svg'"
                        alt="Discord" class="icon-sm" />
                    </button>
                  </div>
                </div>
              </td>

              <td>
                <span class="role-pill" :class="`role-${u.role}`">
                  <img :src="roleIcon(u.role)" :alt="u.role" class="icon-sm" />
                  {{ u.role }}
                </span>
              </td>

              <td>{{ u.total_uploads }}</td>
              <td>{{ u.accepted }}</td>
              <td>{{ u.pending }}</td>
              <td>{{ u.rejected }}</td>
              <td>{{ u.active_thumbnails }}</td>

              <td class="actions-th">
                <button class="btn btn-dark btn-sm" disabled>—</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

    </template>
  </div>

  <LoadingCircle v-if="refreshing" backdrop />
</template>

<style scoped>
.users-page {
  display: flex;
  flex-direction: column;
  gap: 20px;
  min-width: 0;
  width: 100%;
}

.centered-fill {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
}

.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  text-align: center;
  font-size: 1.1rem;
}

.error-img {
  width: 128px;
  height: auto;
}

.panel {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 18px;
  padding: 18px 20px;
  backdrop-filter: blur(10px);
  box-shadow: 0 14px 30px rgba(0, 0, 0, 0.18);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.panel-kicker {
  font-size: 0.74rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.55);
}

.panel-count {
  font-size: 0.88rem;
  color: rgba(255, 255, 255, 0.7);
  white-space: nowrap;
}

.filters-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 12px;
}

.filter-label {
  display: flex;
  flex-direction: column;
  gap: 5px;
  font-size: 0.86rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.8);
}

.ctrl {
  background: rgba(255, 255, 255, 0.07);
  color: #fff;
  border: 1px solid rgba(255, 255, 255, 0.11);
  border-radius: 10px;
  padding: 7px 11px;
  font-size: 0.9rem;
  font-family: inherit;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.ctrl::placeholder {
  color: rgba(255, 255, 255, 0.38);
}

.ctrl:focus {
  outline: none;
  border-color: rgba(78, 159, 255, 0.7);
  box-shadow: 0 0 0 3px rgba(78, 159, 255, 0.12);
}

.ctrl.select {
  cursor: pointer;
  appearance: none;
  padding-right: 2.2rem;
  background-image:
    linear-gradient(45deg, transparent 50%, rgba(255, 255, 255, 0.85) 50%),
    linear-gradient(135deg, rgba(255, 255, 255, 0.85) 50%, transparent 50%);
  background-position:
    calc(100% - 17px) calc(50% - 2px),
    calc(100% - 11px) calc(50% - 2px);
  background-size: 6px 6px, 6px 6px;
  background-repeat: no-repeat;
}

.ctrl.select option {
  background: #13263f;
  color: #fff;
}

.pagination {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding-top: 12px;
  border-top: 1px solid rgba(255, 255, 255, 0.07);
}

.page-info {
  margin: 0 6px;
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.85);
  white-space: nowrap;
}

.page-input {
  width: 130px;
  padding: 7px 10px;
}

.table-wrap {
  overflow-x: auto;
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.06);
  box-shadow: 0 14px 30px rgba(0, 0, 0, 0.18);
}

.users-table {
  width: 100%;
  min-width: 760px;
  border-collapse: collapse;
}

.users-table th,
.users-table td {
  padding: 10px 14px;
  vertical-align: middle;
  white-space: nowrap;
}

.users-table thead th {
  text-align: left;
  color: rgba(255, 255, 255, 0.85);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.users-table tbody td {
  border-top: 1px solid rgba(255, 255, 255, 0.04);
}

.users-table tbody tr:nth-child(even) {
  background: rgba(0, 0, 0, 0.10);
}

.users-table tbody tr:hover {
  background: rgba(255, 255, 255, 0.03);
}

.sort-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
  white-space: nowrap;
}

.sort-icon {
  width: 14px;
  height: 14px;
  opacity: 0.25;
  transition: opacity 0.15s;
}

.sort-icon.active {
  opacity: 1;
}

.user-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.username {
  font-weight: 500;
}

.meta-icons {
  display: flex;
  align-items: center;
  gap: 4px;
}

.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 4px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.05);
  color: #fff;
  cursor: pointer;
  transition: background 0.15s, transform 0.15s, box-shadow 0.15s;
  flex-shrink: 0;
}

.icon-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.2);
}

.icon-btn:active {
  transform: none;
}

.icon-btn:disabled {
  opacity: 0.4;
  cursor: default;
  box-shadow: none;
}

.icon-sm {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.role-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 0.76rem;
  font-weight: 700;
  text-transform: capitalize;
  border: 1px solid rgba(255, 255, 255, 0.08);
  white-space: nowrap;
}

.role-user {
  background: rgba(255, 255, 255, 0.07);
  color: rgba(255, 255, 255, 0.9);
}

.role-verified {
  background: rgba(77, 214, 141, 0.16);
  color: #a4f2c8;
}

.role-moderator {
  background: rgba(102, 204, 255, 0.16);
  color: #b9ecff;
}

.role-admin {
  background: rgba(255, 109, 109, 0.16);
  color: #ffb8b8;
}

.mono {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, 'Roboto Mono', monospace;
}

.actions-th {
  width: 120px;
  text-align: center;
}

.btn-sm {
  padding: 7px 12px;
  font-size: 0.88rem;
  border-radius: 10px;
  min-width: 38px;
  min-height: 36px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.12s, opacity 0.12s;
}

.btn-sm:not(:disabled):hover {
  transform: translateY(-1px);
}

.btn-sm:disabled {
  opacity: 0.5;
}

.nav-icon {
  width: 18px;
  height: 18px;
  object-fit: contain;
}

@media (max-width: 640px) {
  .panel {
    padding: 14px 16px;
  }

  .users-table th,
  .users-table td {
    padding: 9px 10px;
  }
}
</style>