<template>
  <div class="dns-query">
    <h1>DNS 查询工具</h1>

    <el-card class="query-card">
      <el-form :model="queryForm" label-width="100px" @submit.prevent="performQuery">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="域名">
              <el-input
                v-model="queryForm.domain"
                placeholder="example.com"
                clearable
                @keyup.enter="performQuery"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="记录类型">
              <el-select v-model="queryForm.record_type" placeholder="选择记录类型">
                <el-option
                  v-for="type in recordTypes"
                  :key="type.value"
                  :label="type.label"
                  :value="type.value"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="4">
            <el-form-item label=" ">
              <el-button
                type="primary"
                @click="performQuery"
                :loading="querying"
                :disabled="!queryForm.domain"
              >
                查询
              </el-button>
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
    </el-card>

    <!-- Query Result -->
    <el-card v-if="result" class="result-card">
      <template #header>
        <div class="result-header">
          <span>查询结果</span>
          <div class="result-meta">
            <el-tag :type="result.cache_hit ? 'success' : 'info'" size="small">
              {{ result.cache_hit ? '缓存命中' : '缓存未命中' }}
            </el-tag>
            <el-tag type="warning" size="small" v-if="result.rewrite_applied">
              已重写
            </el-tag>
            <el-tag size="small">
              {{ result.response_time_ms }}ms
            </el-tag>
          </div>
        </div>
      </template>

      <!-- Response Metadata -->
      <el-descriptions :column="3" border class="metadata-section">
        <el-descriptions-item label="查询域名">
          {{ result.domain }}
        </el-descriptions-item>
        <el-descriptions-item label="记录类型">
          {{ result.record_type }}
        </el-descriptions-item>
        <el-descriptions-item label="响应码">
          <el-tag :type="getResponseCodeType(result.response_code)">
            {{ result.response_code }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="响应时间">
          {{ result.response_time_ms }} ms
        </el-descriptions-item>
        <el-descriptions-item label="缓存命中">
          {{ result.cache_hit ? '是' : '否' }}
        </el-descriptions-item>
        <el-descriptions-item label="上游服务器">
          {{ result.upstream_used || '-' }}
        </el-descriptions-item>
      </el-descriptions>

      <!-- Records Table -->
      <div class="records-section" v-if="result.records.length > 0">
        <h4>DNS 记录</h4>
        <el-table :data="result.records" stripe>
          <el-table-column prop="name" label="名称" min-width="200" />
          <el-table-column prop="record_type" label="类型" width="100">
            <template #default="{ row }">
              <el-tag>{{ row.record_type }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="value" label="值" min-width="250" />
          <el-table-column prop="ttl" label="TTL" width="100">
            <template #default="{ row }">
              {{ row.ttl }}s
            </template>
          </el-table-column>
        </el-table>
      </div>

      <!-- No Records -->
      <el-empty v-else description="未找到记录" />
    </el-card>

    <!-- Error Display -->
    <el-card v-if="error" class="error-card">
      <el-result icon="error" title="查询失败" :sub-title="error">
        <template #extra>
          <el-button type="primary" @click="error = null">关闭</el-button>
        </template>
      </el-result>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import api from '../api'

interface DnsRecord {
  name: string
  record_type: string
  value: string
  ttl: number
}

interface QueryResult {
  domain: string
  record_type: string
  records: DnsRecord[]
  response_time_ms: number
  cache_hit: boolean
  upstream_used: string | null
  rewrite_applied: boolean
  response_code: string
}

const recordTypes = [
  { value: 'A', label: 'A - IPv4 地址' },
  { value: 'AAAA', label: 'AAAA - IPv6 地址' },
  { value: 'CNAME', label: 'CNAME - 别名记录' },
  { value: 'MX', label: 'MX - 邮件服务器' },
  { value: 'TXT', label: 'TXT - 文本记录' },
  { value: 'PTR', label: 'PTR - 反向解析' },
  { value: 'NS', label: 'NS - 域名服务器' },
  { value: 'SOA', label: 'SOA - 授权起始' },
  { value: 'SRV', label: 'SRV - 服务定位' }
]

const queryForm = reactive({
  domain: '',
  record_type: 'A'
})

const querying = ref(false)
const result = ref<QueryResult | null>(null)
const error = ref<string | null>(null)

function getResponseCodeType(code: string): string {
  if (code === 'NOERROR') return 'success'
  if (code === 'NXDOMAIN') return 'warning'
  return 'danger'
}

async function performQuery() {
  if (!queryForm.domain) {
    ElMessage.warning('请输入域名')
    return
  }

  querying.value = true
  result.value = null
  error.value = null

  try {
    const response = await api.post('/api/dns/query', queryForm)
    result.value = response.data
  } catch (err: any) {
    const message = err.response?.data?.message || '查询失败'
    error.value = message
  } finally {
    querying.value = false
  }
}
</script>

<style scoped>
.dns-query {
  padding: 20px;
}

.dns-query h1 {
  margin-bottom: 20px;
}

.query-card {
  margin-bottom: 20px;
}

.result-card {
  margin-bottom: 20px;
}

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.result-meta {
  display: flex;
  gap: 10px;
}

.metadata-section {
  margin-bottom: 20px;
}

.records-section h4 {
  margin: 20px 0 10px 0;
  color: #303133;
}

.error-card {
  margin-bottom: 20px;
}
</style>
