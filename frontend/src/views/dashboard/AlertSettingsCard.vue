<template>
  <el-card class="alert-settings-card" shadow="never">
    <template #header>
      <div class="card-header">
        <div class="card-title">
          <el-icon><Bell /></el-icon>
          <span>告警通知配置</span>
        </div>
        <el-button type="primary" link @click="fetchSettings" :loading="loading">
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </template>
    
    <div v-loading="loading">
      <p class="section-desc">当系统检测到异常（如高延迟）时，通过 Webhook 发送通知。</p>
      
      <el-form :model="form" label-position="top">
        <el-form-item label="启用告警">
          <el-switch v-model="form.alert_enabled" @change="saveSettings" />
        </el-form-item>
        
        <el-form-item label="Webhook URL">
          <el-input 
            v-model="form.alert_webhook_url" 
            placeholder="https://hooks.slack.com/services/..." 
            :disabled="!form.alert_enabled"
          >
            <template #append>
              <el-button @click="saveSettings" :loading="saving">保存</el-button>
            </template>
          </el-input>
          <div class="form-tip">支持 Slack, Discord, 钉钉, 企业微信等 Webhook 格式。</div>
        </el-form-item>
        
        <el-form-item label="延迟阈值 (ms)">
          <el-input-number 
            v-model="form.alert_latency_threshold_ms" 
            :min="1" 
            :max="10000" 
            :step="10"
            :disabled="!form.alert_enabled"
            @change="saveSettings"
          />
          <div class="form-tip">当平均延迟超过此值时触发告警。</div>
        </el-form-item>

        <el-form-item>
          <el-button 
            type="warning" 
            plain 
            @click="testAlert" 
            :loading="testing"
            :disabled="!form.alert_enabled || !form.alert_webhook_url"
          >
            <el-icon><BellFilled /></el-icon>
            发送测试告警
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { Bell, BellFilled, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import api from '../../api'

interface AlertSettings {
  alert_enabled: boolean
  alert_webhook_url: string
  alert_latency_threshold_ms: number
}

const loading = ref(false)
const saving = ref(false)
const testing = ref(false)

const form = reactive<AlertSettings>({
  alert_enabled: false,
  alert_webhook_url: '',
  alert_latency_threshold_ms: 200
})

async function fetchSettings() {
  loading.value = true
  try {
    const response = await api.get('/api/settings')
    const data = response.data
    form.alert_enabled = data.alert_enabled
    form.alert_webhook_url = data.alert_webhook_url || ''
    form.alert_latency_threshold_ms = data.alert_latency_threshold_ms || 200
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取配置失败')
  } finally {
    loading.value = false
  }
}

async function saveSettings() {
  saving.value = true
  try {
    await api.put('/api/settings', {
      alert_enabled: form.alert_enabled,
      alert_webhook_url: form.alert_webhook_url,
      alert_latency_threshold_ms: form.alert_latency_threshold_ms
    })
    ElMessage.success('设置已保存')
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存设置失败')
  } finally {
    saving.value = false
  }
}

async function testAlert() {
  testing.value = true
  try {
    await api.post('/api/settings/test-alert')
    ElMessage.success('测试告警已发送')
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '发送测试告警失败')
  } finally {
    testing.value = false
  }
}

onMounted(() => {
  fetchSettings()
})
</script>

<style scoped>
.alert-settings-card {
  height: 100%;
  border-radius: 12px;
  border: none;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  color: #303133;
}

.section-desc {
  color: #909399;
  font-size: 14px;
  margin-bottom: 20px;
}

.form-tip {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
</style>
