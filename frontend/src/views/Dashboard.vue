<template>
  <div class="dashboard">
    <!-- 页面标题 -->
    <div class="page-header">
      <div class="header-left">
        <h1>仪表盘</h1>
        <p class="subtitle">实时系统监控</p>
      </div>
      <div class="header-right">
        <el-tag :type="isConnected ? 'success' : 'danger'" size="large" effect="dark">
          <el-icon class="status-icon"><CircleCheck v-if="isConnected" /><CircleClose v-else /></el-icon>
          {{ isConnected ? '实时连接' : '连接断开' }}
        </el-tag>
      </div>
    </div>

    <!-- 核心指标 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="8" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);">
            <el-icon><DataAnalysis /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ currentStats.total_queries || 0 }}</span>
            <span class="stat-label">今日查询总数</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="8" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);">
            <el-icon><Odometer /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ currentStats.qps?.toFixed(1) || 0 }}</span>
            <span class="stat-label">当前 QPS</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="8" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);">
            <el-icon><TrendCharts /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ ((currentStats.cache_hit_rate || 0) * 100).toFixed(1) }}%</span>
            <span class="stat-label">缓存命中率</span>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :sm="8" :lg="6">
        <div class="stat-card">
          <div class="stat-icon" style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);">
            <el-icon><Stopwatch /></el-icon>
          </div>
          <div class="stat-info">
            <span class="stat-value">{{ currentStats.avg_latency_ms?.toFixed(1) || 0 }}<small>ms</small></span>
            <span class="stat-label">平均延迟</span>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 图表区域 -->
    <el-row :gutter="20" class="chart-row">
      <el-col :span="24">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header">
              <span class="header-title">流量趋势 (QPS & Latency)</span>
            </div>
          </template>
          <div class="chart-container">
            <v-chart class="chart" :option="chartOption" autoresize />
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 排行榜 -->
    <el-row :gutter="20" class="rank-row">
      <el-col :xs="24" :md="12">
        <TopDomainsCard />
      </el-col>
      <el-col :xs="24" :md="12">
        <TopClientsCard />
      </el-col>
    </el-row>

    <!-- 上游服务器状态 -->
    <el-row :gutter="20" class="detail-row">
      <el-col :span="24">
        <el-card shadow="never" class="upstream-card">
          <template #header>
            <div class="card-header">
              <span class="header-title">上游服务器状态 ({{ currentStats.healthy_upstreams || 0 }}/{{ currentStats.total_upstreams || 0 }})</span>
            </div>
          </template>
          <div class="upstream-grid">
            <div 
              v-for="upstream in currentStats.upstream_status || []" 
              :key="upstream.id"
              class="upstream-item"
              :class="{ 'is-healthy': upstream.healthy, 'is-down': !upstream.healthy }"
            >
              <div class="upstream-status-dot"></div>
              <div class="upstream-info">
                <div class="upstream-name">{{ upstream.name }}</div>
                <div class="upstream-meta">
                  <span class="latency" v-if="upstream.healthy">{{ upstream.latency_ms }}ms</span>
                  <span class="status-text" v-else>Down</span>
                </div>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { 
  CircleCheck, CircleClose, DataAnalysis, TrendCharts, Odometer, Stopwatch
} from '@element-plus/icons-vue'
import { use } from 'echarts/core'
import { CanvasRenderer } from 'echarts/renderers'
import { LineChart } from 'echarts/charts'
import {
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
} from 'echarts/components'
import VChart from 'vue-echarts'

import TopDomainsCard from './dashboard/TopDomainsCard.vue'
import TopClientsCard from './dashboard/TopClientsCard.vue'

// 注册 ECharts 组件
use([
  CanvasRenderer,
  LineChart,
  TitleComponent,
  TooltipComponent,
  LegendComponent,
  GridComponent
])

// 统计数据接口
interface UpstreamStatus {
  id: number
  name: string
  healthy: boolean
  latency_ms: number
}

interface StatsMessage {
  timestamp: number
  qps: number
  avg_latency_ms: number
  cache_hit_rate: number
  total_queries: number
  healthy_upstreams: number
  total_upstreams: number
  upstream_status: UpstreamStatus[]
}

const isConnected = ref(false)
const currentStats = ref<Partial<StatsMessage>>({})
const timeData = ref<string[]>([])
const qpsData = ref<number[]>([])
const latencyData = ref<number[]>([])

let eventSource: EventSource | null = null

// 图表配置
const chartOption = computed(() => ({
  tooltip: {
    trigger: 'axis',
    axisPointer: {
      type: 'cross'
    }
  },
  legend: {
    data: ['QPS', 'Latency (ms)']
  },
  grid: {
    left: '3%',
    right: '4%',
    bottom: '3%',
    containLabel: true
  },
  xAxis: {
    type: 'category',
    boundaryGap: false,
    data: timeData.value
  },
  yAxis: [
    {
      type: 'value',
      name: 'QPS',
      position: 'left',
      splitLine: {
        show: true,
        lineStyle: {
          type: 'dashed'
        }
      }
    },
    {
      type: 'value',
      name: 'Latency',
      position: 'right',
      splitLine: {
        show: false
      }
    }
  ],
  series: [
    {
      name: 'QPS',
      type: 'line',
      smooth: true,
      showSymbol: false,
      areaStyle: {
        opacity: 0.1,
        color: '#11998e'
      },
      itemStyle: {
        color: '#11998e'
      },
      data: qpsData.value
    },
    {
      name: 'Latency (ms)',
      type: 'line',
      yAxisIndex: 1,
      smooth: true,
      showSymbol: false,
      itemStyle: {
        color: '#f5576c'
      },
      data: latencyData.value
    }
  ]
}))

const MAX_POINTS = 60 // 保留最近 60 秒的数据

function connectSSE() {
  // 注意：在开发和生产环境中 URL 可能不同，这里假设 API 路径是相对的
  const token = localStorage.getItem('token')
  const url = `/api/stats/stream?token=${encodeURIComponent(token || '')}`
  eventSource = new EventSource(url)

  eventSource.onopen = () => {
    isConnected.value = true
  }

  eventSource.onmessage = (event) => {
    try {
      const data: StatsMessage = JSON.parse(event.data)
      currentStats.value = data

      // 更新图表数据
      const now = new Date()
      const timeStr = `${now.getHours()}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`

      timeData.value.push(timeStr)
      qpsData.value.push(data.qps)
      latencyData.value.push(data.avg_latency_ms)

      // 限制数据点数量
      if (timeData.value.length > MAX_POINTS) {
        timeData.value.shift()
        qpsData.value.shift()
        latencyData.value.shift()
      }
    } catch (e) {
      console.error('Failed to parse SSE stats:', e)
    }
  }

  eventSource.onerror = () => {
    isConnected.value = false
    eventSource?.close()
    // 5秒后重连
    setTimeout(connectSSE, 5000)
  }
}

onMounted(() => {
  connectSSE()
})

onUnmounted(() => {
  if (eventSource) {
    eventSource.close()
  }
})
</script>

<style scoped>
.dashboard {
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.header-left h1 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.subtitle {
  margin: 4px 0 0 0;
  font-size: 14px;
  color: #909399;
}

.stats-row {
  margin-bottom: 24px;
}

.stat-card {
  background: #fff;
  border-radius: 12px;
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  transition: transform 0.3s;
  height: 100%;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 28px;
  flex-shrink: 0;
}

.stat-info {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.stat-value small {
  font-size: 14px;
  margin-left: 4px;
  color: #909399;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

.chart-card {
  border-radius: 12px;
  border: none;
  margin-bottom: 24px;
}

.rank-row {
  margin-bottom: 24px;
}

.chart-container {
  height: 350px;
  width: 100%;
}

.chart {
  height: 100%;
  width: 100%;
}

.upstream-card {
  border-radius: 12px;
  border: none;
}

.header-title {
  font-weight: 600;
  font-size: 16px;
  color: #303133;
}

.upstream-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}

.upstream-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: #f8f9fa;
  border-radius: 8px;
  border: 1px solid #ebeef5;
}

.upstream-item.is-healthy {
  border-left: 4px solid #67C23A;
}

.upstream-item.is-down {
  border-left: 4px solid #F56C6C;
}

.upstream-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.is-healthy .upstream-status-dot {
  background-color: #67C23A;
  box-shadow: 0 0 8px rgba(103, 194, 58, 0.4);
}

.is-down .upstream-status-dot {
  background-color: #F56C6C;
  box-shadow: 0 0 8px rgba(245, 108, 108, 0.4);
}

.upstream-name {
  font-weight: 500;
  font-size: 14px;
  color: #303133;
}

.upstream-meta {
  font-size: 12px;
  color: #909399;
}

@media (max-width: 768px) {
  .stat-card {
    margin-bottom: 16px;
  }
}
</style>
