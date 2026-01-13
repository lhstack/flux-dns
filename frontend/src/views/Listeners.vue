<template>
  <div class="listeners">
    <h1>服务监听配置</h1>
    <p class="page-desc">配置 DNS 服务器监听的协议和端口，支持 UDP、DoT、DoH、DoQ、DoH3 等协议。</p>

    <el-row :gutter="20">
      <el-col :span="12" v-for="listener in listeners" :key="listener.protocol">
        <el-card class="listener-card" :class="{ 'is-enabled': listener.enabled }">
          <template #header>
            <div class="card-header">
              <div class="protocol-info">
                <el-tag :type="getProtocolTagType(listener.protocol)" size="large">
                  {{ listener.protocol.toUpperCase() }}
                </el-tag>
                <span class="protocol-desc">{{ listener.description }}</span>
              </div>
              <el-switch
                v-model="listener.enabled"
                @change="toggleListener(listener)"
                :disabled="saving[listener.protocol]"
              />
            </div>
          </template>

          <el-form label-width="100px" size="small">
            <el-form-item label="绑定地址">
              <el-input
                v-model="listener.bind_address"
                placeholder="0.0.0.0"
                :disabled="!listener.enabled"
              />
            </el-form-item>
            <el-form-item label="端口">
              <el-input-number
                v-model="listener.port"
                :min="1"
                :max="65535"
                :disabled="!listener.enabled"
              />
            </el-form-item>
            <template v-if="listener.requires_tls">
              <el-divider content-position="left">TLS 证书配置</el-divider>
              
              <el-form-item label="证书状态">
                <el-tag :type="listener.has_tls_cert ? 'success' : 'danger'" size="small">
                  {{ listener.has_tls_cert ? '已配置' : '未配置' }}
                </el-tag>
              </el-form-item>
              
              <el-form-item label="证书">
                <div class="cert-input-group">
                  <el-button 
                    type="primary" 
                    size="small" 
                    @click="openCertDialog(listener, 'cert')"
                    :disabled="!listener.enabled"
                  >
                    {{ listener.has_tls_cert ? '更新证书' : '配置证书' }}
                  </el-button>
                  <el-button 
                    v-if="listener.has_tls_cert"
                    type="danger" 
                    size="small" 
                    @click="clearCert(listener, 'cert')"
                    :disabled="!listener.enabled"
                  >
                    清除
                  </el-button>
                </div>
              </el-form-item>

              <el-form-item label="私钥状态">
                <el-tag :type="listener.has_tls_key ? 'success' : 'danger'" size="small">
                  {{ listener.has_tls_key ? '已配置' : '未配置' }}
                </el-tag>
              </el-form-item>
              
              <el-form-item label="私钥">
                <div class="cert-input-group">
                  <el-button 
                    type="primary" 
                    size="small" 
                    @click="openCertDialog(listener, 'key')"
                    :disabled="!listener.enabled"
                  >
                    {{ listener.has_tls_key ? '更新私钥' : '配置私钥' }}
                  </el-button>
                  <el-button 
                    v-if="listener.has_tls_key"
                    type="danger" 
                    size="small" 
                    @click="clearCert(listener, 'key')"
                    :disabled="!listener.enabled"
                  >
                    清除
                  </el-button>
                </div>
              </el-form-item>

              <el-alert
                v-if="listener.enabled && (!listener.has_tls_cert || !listener.has_tls_key)"
                type="warning"
                :closable="false"
                show-icon
                class="tls-warning"
              >
                需要配置 TLS 证书和私钥才能启动此服务
              </el-alert>
            </template>
            <el-form-item>
              <el-button
                type="primary"
                @click="saveListener(listener)"
                :loading="saving[listener.protocol]"
                :disabled="!listener.enabled"
              >
                保存配置
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
    </el-row>

    <el-alert
      type="info"
      title="提示"
      description="修改监听配置后需要重启服务才能生效。"
      show-icon
      :closable="false"
      class="notice-alert"
    />

    <!-- Certificate Dialog -->
    <el-dialog
      v-model="certDialogVisible"
      :title="certDialogTitle"
      width="600px"
      :close-on-click-modal="false"
    >
      <el-tabs v-model="certInputMode">
        <el-tab-pane label="粘贴内容" name="paste">
          <el-input
            v-model="certContent"
            type="textarea"
            :rows="12"
            :placeholder="certPlaceholder"
          />
        </el-tab-pane>
        <el-tab-pane label="上传文件" name="upload">
          <el-upload
            class="cert-upload"
            drag
            :auto-upload="false"
            :show-file-list="false"
            @change="handleFileChange"
            accept=".pem,.crt,.key,.cer"
          >
            <el-icon class="el-icon--upload"><upload-filled /></el-icon>
            <div class="el-upload__text">
              拖拽文件到此处，或 <em>点击上传</em>
            </div>
            <template #tip>
              <div class="el-upload__tip">
                支持 .pem, .crt, .key, .cer 格式
              </div>
            </template>
          </el-upload>
          <el-input
            v-if="certContent"
            v-model="certContent"
            type="textarea"
            :rows="8"
            readonly
            class="uploaded-content"
          />
        </el-tab-pane>
      </el-tabs>
      <template #footer>
        <el-button @click="certDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveCert" :loading="savingCert">
          保存
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { UploadFilled } from '@element-plus/icons-vue'
import api from '../api'

interface Listener {
  protocol: string
  enabled: boolean
  bind_address: string
  port: number
  has_tls_cert: boolean
  has_tls_key: boolean
  requires_tls: boolean
  description: string
}

const listeners = ref<Listener[]>([])
const loading = ref(false)
const saving = reactive<Record<string, boolean>>({})

// Certificate dialog
const certDialogVisible = ref(false)
const certInputMode = ref('paste')
const certContent = ref('')
const certType = ref<'cert' | 'key'>('cert')
const currentListener = ref<Listener | null>(null)
const savingCert = ref(false)

const certDialogTitle = computed(() => {
  if (!currentListener.value) return ''
  const protocol = currentListener.value.protocol.toUpperCase()
  return certType.value === 'cert' ? `${protocol} - 配置 TLS 证书` : `${protocol} - 配置 TLS 私钥`
})

const certPlaceholder = computed(() => {
  return certType.value === 'cert' 
    ? '-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----'
    : '-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----'
})

function getProtocolTagType(protocol: string): string {
  const types: Record<string, string> = {
    udp: '',
    dot: 'success',
    doh: 'warning',
    doq: 'danger',
    doh3: 'info'
  }
  return types[protocol] || ''
}

async function fetchListeners() {
  loading.value = true
  try {
    const response = await api.get('/api/listeners')
    listeners.value = response.data.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取监听配置失败')
  } finally {
    loading.value = false
  }
}

async function toggleListener(listener: Listener) {
  saving[listener.protocol] = true
  try {
    await api.put(`/api/listeners/${listener.protocol}`, {
      enabled: listener.enabled
    })
    if (listener.enabled && listener.requires_tls && (!listener.has_tls_cert || !listener.has_tls_key)) {
      ElMessage.warning(`${listener.protocol.toUpperCase()} 已启用，请配置 TLS 证书`)
    } else {
      ElMessage.success(listener.enabled ? `${listener.protocol.toUpperCase()} 已启用` : `${listener.protocol.toUpperCase()} 已禁用`)
    }
  } catch (error: any) {
    listener.enabled = !listener.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  } finally {
    saving[listener.protocol] = false
  }
}

async function saveListener(listener: Listener) {
  saving[listener.protocol] = true
  try {
    const response = await api.put(`/api/listeners/${listener.protocol}`, {
      enabled: listener.enabled,
      bind_address: listener.bind_address,
      port: listener.port
    })
    // Update local state
    Object.assign(listener, response.data)
    ElMessage.success(`${listener.protocol.toUpperCase()} 配置已保存`)
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存失败')
  } finally {
    saving[listener.protocol] = false
  }
}

function openCertDialog(listener: Listener, type: 'cert' | 'key') {
  currentListener.value = listener
  certType.value = type
  certContent.value = ''
  certInputMode.value = 'paste'
  certDialogVisible.value = true
}

function handleFileChange(file: any) {
  const reader = new FileReader()
  reader.onload = (e) => {
    certContent.value = e.target?.result as string
  }
  reader.readAsText(file.raw)
}

async function saveCert() {
  if (!currentListener.value || !certContent.value.trim()) {
    ElMessage.warning('请输入或上传证书内容')
    return
  }

  savingCert.value = true
  try {
    const payload: any = {}
    if (certType.value === 'cert') {
      payload.tls_cert = certContent.value
    } else {
      payload.tls_key = certContent.value
    }

    const response = await api.put(`/api/listeners/${currentListener.value.protocol}`, payload)
    
    // Update local state
    const idx = listeners.value.findIndex(l => l.protocol === currentListener.value?.protocol)
    if (idx !== -1 && listeners.value[idx]) {
      Object.assign(listeners.value[idx], response.data)
    }

    ElMessage.success(certType.value === 'cert' ? '证书已保存' : '私钥已保存')
    certDialogVisible.value = false
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '保存失败')
  } finally {
    savingCert.value = false
  }
}

async function clearCert(listener: Listener, type: 'cert' | 'key') {
  try {
    await ElMessageBox.confirm(
      `确定要清除 ${listener.protocol.toUpperCase()} 的${type === 'cert' ? '证书' : '私钥'}吗？`,
      '确认',
      { type: 'warning' }
    )

    saving[listener.protocol] = true
    const payload: any = {}
    if (type === 'cert') {
      payload.tls_cert = ''
    } else {
      payload.tls_key = ''
    }

    const response = await api.put(`/api/listeners/${listener.protocol}`, payload)
    Object.assign(listener, response.data)
    ElMessage.success('已清除')
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '操作失败')
    }
  } finally {
    saving[listener.protocol] = false
  }
}

onMounted(() => {
  fetchListeners()
})
</script>

<style scoped>
.listeners {
  padding: 20px;
}

.listeners h1 {
  margin-bottom: 10px;
}

.page-desc {
  color: #909399;
  margin-bottom: 20px;
}

.listener-card {
  margin-bottom: 20px;
  transition: all 0.3s;
}

.listener-card.is-enabled {
  border-color: #67C23A;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.protocol-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.protocol-desc {
  color: #606266;
  font-size: 14px;
}

.notice-alert {
  margin-top: 20px;
}

.cert-input-group {
  display: flex;
  gap: 8px;
}

.tls-warning {
  margin-bottom: 16px;
}

.cert-upload {
  width: 100%;
}

.uploaded-content {
  margin-top: 16px;
}

:deep(.el-divider__text) {
  font-size: 12px;
  color: #909399;
}

:deep(.el-upload-dragger) {
  width: 100%;
}
</style>
