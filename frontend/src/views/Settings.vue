<template>
  <div class="settings">
    <h1>系统设置</h1>

    <el-row :gutter="20">
      <!-- Query Strategy -->
      <el-col :span="12">
        <el-card>
          <template #header>
            <span>查询策略</span>
          </template>
          <div v-loading="loadingStrategy">
            <el-form label-width="100px">
              <el-form-item label="当前策略">
                <el-tag type="primary" size="large">
                  {{ getStrategyLabel(currentStrategy.strategy) }}
                </el-tag>
              </el-form-item>
              <el-form-item label="策略描述">
                <span class="strategy-desc">{{ currentStrategy.description }}</span>
              </el-form-item>
              <el-divider />
              <el-form-item label="选择策略">
                <el-radio-group v-model="selectedStrategy" class="strategy-radio-group">
                  <el-radio
                    v-for="strategy in availableStrategies"
                    :key="strategy.name"
                    :value="strategy.name"
                    class="strategy-radio"
                  >
                    <div class="strategy-option">
                      <span class="strategy-name">{{ getStrategyLabel(strategy.name) }}</span>
                      <span class="strategy-option-desc">{{ strategy.description }}</span>
                    </div>
                  </el-radio>
                </el-radio-group>
              </el-form-item>
              <el-form-item>
                <el-button
                  type="primary"
                  @click="saveStrategy"
                  :loading="savingStrategy"
                  :disabled="selectedStrategy === currentStrategy.strategy"
                >
                  保存策略
                </el-button>
              </el-form-item>
            </el-form>
          </div>
        </el-card>
      </el-col>

      <!-- System Status -->
      <el-col :span="12">
        <el-card>
          <template #header>
            <div class="card-header">
              <span>系统状态</span>
              <el-button type="primary" link @click="fetchStatus">
                <el-icon><Refresh /></el-icon>
                刷新
              </el-button>
            </div>
          </template>
          <div v-loading="loadingStatus">
            <el-descriptions :column="1" border>
              <el-descriptions-item label="运行状态">
                <el-tag :type="status.status === 'running' ? 'success' : 'danger'">
                  {{ status.status === 'running' ? '运行中' : '异常' }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="运行时间">
                {{ formatUptime(status.uptime_seconds) }}
              </el-descriptions-item>
              <el-descriptions-item label="缓存条目">
                {{ status.cache?.entries || 0 }}
              </el-descriptions-item>
              <el-descriptions-item label="缓存命中率">
                {{ ((status.cache?.hit_rate || 0) * 100).toFixed(1) }}%
              </el-descriptions-item>
              <el-descriptions-item label="总查询数">
                {{ status.query?.total_queries || 0 }}
              </el-descriptions-item>
              <el-descriptions-item label="今日查询">
                {{ status.query?.queries_today || 0 }}
              </el-descriptions-item>
              <el-descriptions-item label="上游服务器">
                {{ status.upstreams?.healthy || 0 }} / {{ status.upstreams?.total || 0 }} 健康
              </el-descriptions-item>
              <el-descriptions-item label="查询策略">
                {{ getStrategyLabel(status.strategy) }}
              </el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- Health Check -->
    <el-card class="health-card">
      <template #header>
        <div class="card-header">
          <span>健康检查</span>
          <el-button type="primary" link @click="fetchHealth">
            <el-icon><Refresh /></el-icon>
            检查
          </el-button>
        </div>
      </template>
      <div v-loading="loadingHealth">
        <el-row :gutter="20">
          <el-col :span="8">
            <div class="health-item">
              <el-icon :size="40" :color="health.database ? '#67C23A' : '#F56C6C'">
                <svg v-if="health.database" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/><path fill="currentColor" d="M745.344 361.344a32 32 0 0 1 45.312 45.312l-288 288a32 32 0 0 1-45.312 0l-160-160a32 32 0 1 1 45.312-45.312L480 626.752l265.344-265.408z"/></svg>
                <svg v-else viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="m466.752 512-90.496-90.496a32 32 0 0 1 45.248-45.248L512 466.752l90.496-90.496a32 32 0 1 1 45.248 45.248L557.248 512l90.496 90.496a32 32 0 1 1-45.248 45.248L512 557.248l-90.496 90.496a32 32 0 0 1-45.248-45.248L466.752 512z"/><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/></svg>
              </el-icon>
              <div class="health-label">数据库</div>
              <div class="health-status">{{ health.database ? '正常' : '异常' }}</div>
            </div>
          </el-col>
          <el-col :span="8">
            <div class="health-item">
              <el-icon :size="40" :color="health.cache ? '#67C23A' : '#F56C6C'">
                <svg v-if="health.cache" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/><path fill="currentColor" d="M745.344 361.344a32 32 0 0 1 45.312 45.312l-288 288a32 32 0 0 1-45.312 0l-160-160a32 32 0 1 1 45.312-45.312L480 626.752l265.344-265.408z"/></svg>
                <svg v-else viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="m466.752 512-90.496-90.496a32 32 0 0 1 45.248-45.248L512 466.752l90.496-90.496a32 32 0 1 1 45.248 45.248L557.248 512l90.496 90.496a32 32 0 1 1-45.248 45.248L512 557.248l-90.496 90.496a32 32 0 0 1-45.248-45.248L466.752 512z"/><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/></svg>
              </el-icon>
              <div class="health-label">缓存</div>
              <div class="health-status">{{ health.cache ? '正常' : '异常' }}</div>
            </div>
          </el-col>
          <el-col :span="8">
            <div class="health-item">
              <el-icon :size="40" :color="health.upstreams ? '#67C23A' : '#F56C6C'">
                <svg v-if="health.upstreams" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/><path fill="currentColor" d="M745.344 361.344a32 32 0 0 1 45.312 45.312l-288 288a32 32 0 0 1-45.312 0l-160-160a32 32 0 1 1 45.312-45.312L480 626.752l265.344-265.408z"/></svg>
                <svg v-else viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg"><path fill="currentColor" d="m466.752 512-90.496-90.496a32 32 0 0 1 45.248-45.248L512 466.752l90.496-90.496a32 32 0 1 1 45.248 45.248L557.248 512l90.496 90.496a32 32 0 1 1-45.248 45.248L512 557.248l-90.496 90.496a32 32 0 0 1-45.248-45.248L466.752 512z"/><path fill="currentColor" d="M512 896a384 384 0 1 0 0-768 384 384 0 0 0 0 768m0 64a448 448 0 1 1 0-896 448 448 0 0 1 0 896"/></svg>
              </el-icon>
              <div class="health-label">上游服务器</div>
              <div class="health-status">{{ health.upstreams ? '正常' : '无可用' }}</div>
            </div>
          </el-col>
        </el-row>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import api from '../api'

interface Strategy {
  strategy: string
  description: string
}

interface StrategyInfo {
  name: string
  description: string
}

interface SystemStatus {
  status: string
  uptime_seconds: number
  cache: {
    entries: number
    hits: number
    misses: number
    hit_rate: number
    default_ttl: number
    max_entries: number
  }
  query: {
    total_queries: number
    cache_hits: number
    queries_today: number
  }
  upstreams: {
    total: number
    healthy: number
    servers: any[]
  }
  strategy: string
}

interface HealthCheck {
  status: string
  database: boolean
  cache: boolean
  upstreams: boolean
}

const currentStrategy = ref<Strategy>({
  strategy: '',
  description: ''
})
const selectedStrategy = ref('')
const availableStrategies = ref<StrategyInfo[]>([])
const loadingStrategy = ref(false)
const savingStrategy = ref(false)

const status = ref<SystemStatus>({
  status: '',
  uptime_seconds: 0,
  cache: { entries: 0, hits: 0, misses: 0, hit_rate: 0, default_ttl: 60, max_entries: 10000 },
  query: { total_queries: 0, cache_hits: 0, queries_today: 0 },
  upstreams: { total: 0, healthy: 0, servers: [] },
  strategy: ''
})
const loadingStatus = ref(false)

const health = ref<HealthCheck>({
  status: '',
  database: false,
  cache: false,
  upstreams: false
})
const loadingHealth = ref(false)

const strategyLabels: Record<string, string> = {
  concurrent: '并发查询',
  fastest: '最快响应',
  round_robin: '轮询',
  random: '随机'
}

function getStrategyLabel(strategy: string): string {
  return strategyLabels[strategy] || strategy
}

function formatUptime(seconds: number): string {
  if (!seconds) return '-'
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  const parts = []
  if (days > 0) parts.push(`${days}天`)
  if (hours > 0) parts.push(`${hours}小时`)
  if (minutes > 0) parts.push(`${minutes}分钟`)
  if (secs > 0 || parts.length === 0) parts.push(`${secs}秒`)

  return parts.join(' ')
}

async function fetchStrategy() {
  loadingStrategy.value = true
  try {
    const [currentRes, availableRes] = await Promise.all([
      api.get('/api/strategy'),
      api.get('/api/strategy/available')
    ])
    currentStrategy.value = currentRes.data
    selectedStrategy.value = currentRes.data.strategy
    availableStrategies.value = availableRes.data.strategies
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取策略配置失败')
  } finally {
    loadingStrategy.value = false
  }
}

async function saveStrategy() {
  savingStrategy.value = true
  try {
    const response = await api.put('/api/strategy', { strategy: selectedStrategy.value })
    currentStrategy.value = response.data
    ElMessage.success('策略已更新')
    fetchStatus()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存策略失败')
  } finally {
    savingStrategy.value = false
  }
}

async function fetchStatus() {
  loadingStatus.value = true
  try {
    const response = await api.get('/api/status')
    status.value = response.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取系统状态失败')
  } finally {
    loadingStatus.value = false
  }
}

async function fetchHealth() {
  loadingHealth.value = true
  try {
    const response = await api.get('/api/status/health')
    health.value = response.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '健康检查失败')
  } finally {
    loadingHealth.value = false
  }
}

onMounted(() => {
  fetchStrategy()
  fetchStatus()
  fetchHealth()
})
</script>

<style scoped>
.settings {
  padding: 20px;
}

.settings h1 {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.strategy-desc {
  color: #909399;
  font-size: 14px;
}

.strategy-radio-group {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
}

.strategy-radio {
  height: auto !important;
  padding: 8px 0;
  margin-right: 0 !important;
  width: 100%;
}

.strategy-radio :deep(.el-radio__input) {
  margin-top: 2px;
}

.strategy-radio :deep(.el-radio__label) {
  padding-left: 8px;
  flex: 1;
}

.strategy-option {
  display: inline-flex;
  flex-direction: column;
  vertical-align: top;
}

.strategy-name {
  font-weight: 500;
  color: #303133;
  line-height: 1.4;
}

.strategy-option-desc {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
  line-height: 1.4;
}

.health-card {
  margin-top: 20px;
}

.health-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px;
}

.health-label {
  margin-top: 10px;
  font-size: 16px;
  font-weight: 500;
  color: #303133;
}

.health-status {
  margin-top: 5px;
  font-size: 14px;
  color: #909399;
}
</style>
