<template>
  <div class="dashboard">
    <h1>仪表盘</h1>
    <el-row :gutter="20">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>总查询数</template>
          <div class="stat-value">{{ stats.query?.total_queries || 0 }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>缓存命中率</template>
          <div class="stat-value">{{ ((stats.cache?.hit_rate || 0) * 100).toFixed(1) }}%</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>缓存条目</template>
          <div class="stat-value">{{ stats.cache?.entries || 0 }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>运行时间</template>
          <div class="stat-value">{{ formatUptime(stats.uptime_seconds || 0) }}</div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" style="margin-top: 20px;">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>上游服务器</template>
          <div class="upstream-stats">
            <div class="stat-item">
              <span class="label">总数</span>
              <span class="value">{{ stats.upstreams?.total || 0 }}</span>
            </div>
            <div class="stat-item">
              <span class="label">健康</span>
              <span class="value healthy">{{ stats.upstreams?.healthy || 0 }}</span>
            </div>
            <div class="stat-item">
              <span class="label">查询策略</span>
              <span class="value">{{ strategyName }}</span>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>今日统计</template>
          <div class="upstream-stats">
            <div class="stat-item">
              <span class="label">今日查询</span>
              <span class="value">{{ stats.query?.queries_today || 0 }}</span>
            </div>
            <div class="stat-item">
              <span class="label">缓存命中</span>
              <span class="value">{{ stats.query?.cache_hits || 0 }}</span>
            </div>
            <div class="stat-item">
              <span class="label">服务状态</span>
              <el-tag :type="stats.status === 'running' ? 'success' : 'danger'" size="small">
                {{ stats.status === 'running' ? '运行中' : '异常' }}
              </el-tag>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import api from '../api'

interface Stats {
  status?: string
  uptime_seconds?: number
  cache?: {
    entries: number
    hits: number
    misses: number
    hit_rate: number
    default_ttl: number
    max_entries: number
  }
  query?: {
    total_queries: number
    cache_hits: number
    queries_today: number
  }
  upstreams?: {
    total: number
    healthy: number
  }
  strategy?: string
}

const stats = ref<Stats>({})

const strategyName = computed(() => {
  const names: Record<string, string> = {
    'concurrent': '并发查询',
    'round_robin': '轮询',
    'random': '随机',
    'fastest': '最快响应'
  }
  return names[stats.value.strategy || ''] || stats.value.strategy || '-'
})

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  
  if (days > 0) return `${days}天 ${hours}小时`
  if (hours > 0) return `${hours}小时 ${minutes}分钟`
  return `${minutes}分钟`
}

async function fetchStats() {
  try {
    const response = await api.get('/api/status')
    stats.value = response.data
  } catch {
    // Handle error silently for now
  }
}

onMounted(() => {
  fetchStats()
  // Auto refresh every 30 seconds
  setInterval(fetchStats, 30000)
})
</script>

<style scoped>
.dashboard h1 {
  margin-bottom: 20px;
}

.stat-value {
  font-size: 32px;
  font-weight: bold;
  color: #409EFF;
  text-align: center;
}

.upstream-stats {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.stat-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #eee;
}

.stat-item:last-child {
  border-bottom: none;
}

.stat-item .label {
  color: #909399;
}

.stat-item .value {
  font-weight: bold;
  color: #303133;
}

.stat-item .value.healthy {
  color: #67C23A;
}
</style>
