<template>
  <div class="dns-records">
    <div class="page-header">
      <h1>DNS 记录管理</h1>
      <el-button type="primary" @click="openCreateDialog">
        <el-icon><Plus /></el-icon>
        添加记录
      </el-button>
    </div>

    <el-card>
      <el-table :data="records" v-loading="loading" stripe>
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="name" label="域名" min-width="180" />
        <el-table-column prop="record_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag>{{ row.record_type }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="value" label="值" min-width="200" />
        <el-table-column prop="ttl" label="TTL" width="100" />
        <el-table-column prop="priority" label="优先级" width="100" />
        <el-table-column prop="enabled" label="状态" width="100">
          <template #default="{ row }">
            <el-switch
              v-model="row.enabled"
              @change="toggleEnabled(row)"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link @click="openEditDialog(row)">
              编辑
            </el-button>
            <el-button type="danger" link @click="confirmDelete(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑记录' : '添加记录'"
      width="500px"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="80px"
      >
        <el-form-item label="域名" prop="name">
          <el-input v-model="formData.name" placeholder="example.com" />
        </el-form-item>
        <el-form-item label="类型" prop="record_type">
          <el-select v-model="formData.record_type" placeholder="选择记录类型">
            <el-option
              v-for="type in recordTypes"
              :key="type"
              :label="type"
              :value="type"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="值" prop="value">
          <el-input
            v-model="formData.value"
            :placeholder="getValuePlaceholder(formData.record_type)"
          />
        </el-form-item>
        <el-form-item label="TTL" prop="ttl">
          <el-input-number v-model="formData.ttl" :min="0" :max="86400" />
        </el-form-item>
        <el-form-item label="优先级" prop="priority">
          <el-input-number v-model="formData.priority" :min="0" />
        </el-form-item>
        <el-form-item label="启用" prop="enabled">
          <el-switch v-model="formData.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitForm" :loading="submitting">
          {{ isEditing ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import api from '../api'

interface DnsRecord {
  id: number
  name: string
  record_type: string
  value: string
  ttl: number
  priority: number
  enabled: boolean
  created_at: string
  updated_at: string
}

const records = ref<DnsRecord[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const isEditing = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editingId = ref<number | null>(null)

const recordTypes = ['A', 'AAAA', 'CNAME', 'MX', 'TXT', 'PTR', 'NS', 'SOA', 'SRV']

const formData = reactive({
  name: '',
  record_type: 'A',
  value: '',
  ttl: 300,
  priority: 0,
  enabled: true
})

const formRules: FormRules = {
  name: [
    { required: true, message: '请输入域名', trigger: 'blur' },
    { max: 255, message: '域名长度不能超过255个字符', trigger: 'blur' }
  ],
  record_type: [
    { required: true, message: '请选择记录类型', trigger: 'change' }
  ],
  value: [
    { required: true, message: '请输入记录值', trigger: 'blur' }
  ],
  ttl: [
    { required: true, message: '请输入TTL', trigger: 'blur' }
  ]
}

function getValuePlaceholder(type: string): string {
  const placeholders: Record<string, string> = {
    A: '192.168.1.1',
    AAAA: '2001:db8::1',
    CNAME: 'target.example.com',
    MX: 'mail.example.com',
    TXT: 'v=spf1 include:example.com ~all',
    PTR: 'host.example.com',
    NS: 'ns1.example.com',
    SOA: 'ns1.example.com admin.example.com',
    SRV: '10 5 5060 sipserver.example.com'
  }
  return placeholders[type] || ''
}

async function fetchRecords() {
  loading.value = true
  try {
    const response = await api.get('/api/records')
    records.value = response.data.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取记录失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  formData.name = ''
  formData.record_type = 'A'
  formData.value = ''
  formData.ttl = 300
  formData.priority = 0
  formData.enabled = true
  editingId.value = null
}

function openCreateDialog() {
  isEditing.value = false
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(record: DnsRecord) {
  isEditing.value = true
  editingId.value = record.id
  formData.name = record.name
  formData.record_type = record.record_type
  formData.value = record.value
  formData.ttl = record.ttl
  formData.priority = record.priority
  formData.enabled = record.enabled
  dialogVisible.value = true
}

async function submitForm() {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitting.value = true
    try {
      if (isEditing.value && editingId.value) {
        await api.put(`/api/records/${editingId.value}`, formData)
        ElMessage.success('记录更新成功')
      } else {
        await api.post('/api/records', formData)
        ElMessage.success('记录创建成功')
      }
      dialogVisible.value = false
      fetchRecords()
    } catch (error: any) {
      const message = error.response?.data?.message || '操作失败'
      ElMessage.error(message)
    } finally {
      submitting.value = false
    }
  })
}

async function toggleEnabled(record: DnsRecord) {
  try {
    await api.put(`/api/records/${record.id}`, { enabled: record.enabled })
    ElMessage.success(record.enabled ? '记录已启用' : '记录已禁用')
  } catch (error: any) {
    record.enabled = !record.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}

async function confirmDelete(record: DnsRecord) {
  try {
    await ElMessageBox.confirm(
      `确定要删除记录 "${record.name}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await api.delete(`/api/records/${record.id}`)
    ElMessage.success('记录删除成功')
    fetchRecords()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchRecords()
})
</script>

<style scoped>
.dns-records {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h1 {
  margin: 0;
}
</style>
