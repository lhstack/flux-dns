<template>
  <div class="cache-management">
    <h1>缓存管理</h1>

    <el-row :gutter="20">
      <!-- Cache Statistics -->
      <el-col :span="12">
        <el-card>
          <template #header>
            <div class="card-header">
              <span>缓存统计</span>
              <el-button type="primary" link @click="fetchStats">
                <el-icon><Refresh /></el-icon>
                刷新
              </el-button>
            </div>
          </template>
          <el-descriptions :column="1" border v-loading="loadingStats">
            <el-descriptions-item label="缓存条目">
              {{ stats.entries }}
            </el-descriptions-item>
            <el-descriptions-item label="命中次数">
              {{ stats.hits }}
            </el-descriptions-item>
            <el-descriptions-item label="未命中次数">
              {{ stats.misses }}
            </el-descriptions-item>
            <el-descriptions-item label="命中率">
              <el-progress
                :percentage="stats.hit_rate * 100"
                :format="formatHitRate"
                :stroke-width="15"
              />
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>

      <!-- Cache Configuration -->
      <el-col :span="12">
        <el-card>
          <template #header>
            <span>缓存配置</span>
          </template>
          <el-form
            ref="configFormRef"
            :model="configForm"
            label-width="120px"
            v-loading="loadingConfig"
          >
            <el-form-item label="默认 TTL">
              <el-input-number
                v-model="configForm.default_ttl"
                :min="1"
                :max="604800"
                :step="60"
              />
              <span class="unit-label">秒</span>
            </el-form-item>
            <el-form-item label="最大条目数">
              <el-input-number
                v-model="configForm.max_entries"
                :min="1"
                :max="1000000"
                :step="1000"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="saveConfig" :loading="savingConfig">
                保存配置
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
    </el-row>

    <!-- Cache Operations -->
    <el-card class="operations-card">
      <template #header>
        <span>缓存操作</span>
      </template>
      <el-row :gutter="20">
        <el-col :span="12">
          <div class="operation-section">
            <h4>清除指定域名缓存</h4>
            <el-input
              v-model="clearDomain"
              placeholder="输入域名，如 example.com"
              class="domain-input"
            >
              <template #append>
                <el-button
                  type="primary"
                  @click="clearDomainCache"
                  :loading="clearingDomain"
                  :disabled="!clearDomain"
                >
                  清除
                </el-button>
              </template>
            </el-input>
          </div>
        </el-col>
        <el-col :span="12">
          <div class="operation-section">
            <h4>清除全部缓存</h4>
            <p class="operation-desc">清除所有缓存条目，此操作不可撤销。</p>
            <el-button type="danger" @click="confirmClearAll" :loading="clearingAll">
              清除全部缓存
            </el-button>
          </div>
        </el-col>
      </el-row>
      <el-divider />
      <el-row>
        <el-col :span="24">
          <div class="operation-section">
            <h4>清理过期缓存</h4>
            <p class="operation-desc">清理所有已过期的缓存条目，释放内存空间。</p>
            <el-button type="warning" @click="cleanupExpired" :loading="cleaningUp">
              清理过期缓存
            </el-button>
          </div>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import api from '../api'

interface CacheStats {
  hits: number
  misses: number
  entries: number
  hit_rate: number
}

interface CacheConfig {
  default_ttl: number
  max_entries: number
}

const stats = ref<CacheStats>({
  hits: 0,
  misses: 0,
  entries: 0,
  hit_rate: 0
})

const configForm = reactive<CacheConfig>({
  default_ttl: 60,
  max_entries: 10000
})

const loadingStats = ref(false)
const loadingConfig = ref(false)
const savingConfig = ref(false)
const clearDomain = ref('')
const clearingDomain = ref(false)
const clearingAll = ref(false)
const cleaningUp = ref(false)

function formatHitRate(percentage: number): string {
  return `${percentage.toFixed(1)}%`
}

async function fetchStats() {
  loadingStats.value = true
  try {
    const response = await api.get('/api/cache/stats')
    stats.value = response.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取缓存统计失败')
  } finally {
    loadingStats.value = false
  }
}

async function fetchConfig() {
  loadingConfig.value = true
  try {
    const response = await api.get('/api/cache/config')
    configForm.default_ttl = response.data.default_ttl
    configForm.max_entries = response.data.max_entries
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取缓存配置失败')
  } finally {
    loadingConfig.value = false
  }
}

async function saveConfig() {
  savingConfig.value = true
  try {
    await api.put('/api/cache/config', configForm)
    ElMessage.success('缓存配置已保存')
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存配置失败')
  } finally {
    savingConfig.value = false
  }
}

async function clearDomainCache() {
  if (!clearDomain.value) return
  
  clearingDomain.value = true
  try {
    await api.post(`/api/cache/clear/${encodeURIComponent(clearDomain.value)}`)
    ElMessage.success(`已清除域名 ${clearDomain.value} 的缓存`)
    clearDomain.value = ''
    fetchStats()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '清除缓存失败')
  } finally {
    clearingDomain.value = false
  }
}

async function confirmClearAll() {
  try {
    await ElMessageBox.confirm(
      '确定要清除全部缓存吗？此操作不可撤销。',
      '确认清除',
      {
        confirmButtonText: '清除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    clearingAll.value = true
    await api.post('/api/cache/clear')
    ElMessage.success('全部缓存已清除')
    fetchStats()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '清除缓存失败')
    }
  } finally {
    clearingAll.value = false
  }
}

async function cleanupExpired() {
  cleaningUp.value = true
  try {
    const response = await api.post('/api/cache/cleanup')
    ElMessage.success(`过期缓存已清理，剩余 ${response.data.remaining_entries} 条`)
    fetchStats()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '清理缓存失败')
  } finally {
    cleaningUp.value = false
  }
}

onMounted(() => {
  fetchStats()
  fetchConfig()
})
</script>

<style scoped>
.cache-management {
  padding: 20px;
}

.cache-management h1 {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.operations-card {
  margin-top: 20px;
}

.operation-section {
  padding: 10px 0;
}

.operation-section h4 {
  margin: 0 0 10px 0;
  color: #303133;
}

.operation-desc {
  margin: 0 0 15px 0;
  color: #909399;
  font-size: 14px;
}

.domain-input {
  max-width: 400px;
}

.unit-label {
  margin-left: 10px;
  color: #999;
}
</style>
