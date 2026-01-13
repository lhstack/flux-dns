<template>
  <div class="rewrite-rules">
    <div class="page-header">
      <h1>重写规则管理</h1>
      <el-button type="primary" @click="openCreateDialog">
        <el-icon><Plus /></el-icon>
        添加规则
      </el-button>
    </div>

    <el-card>
      <el-table :data="rules" v-loading="loading" stripe>
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="pattern" label="匹配模式" min-width="200" />
        <el-table-column prop="match_type" label="匹配类型" width="120">
          <template #default="{ row }">
            <el-tag :type="getMatchTypeTag(row.match_type)">
              {{ getMatchTypeLabel(row.match_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="action_type" label="动作" width="120">
          <template #default="{ row }">
            <el-tag :type="getActionTypeTag(row.action_type)">
              {{ getActionTypeLabel(row.action_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="action_value" label="动作值" min-width="150">
          <template #default="{ row }">
            {{ row.action_value || '-' }}
          </template>
        </el-table-column>
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
      :title="isEditing ? '编辑规则' : '添加规则'"
      width="550px"
    >
      <el-form
        ref="formRef"
        :model="formData"
        :rules="formRules"
        label-width="100px"
      >
        <el-form-item label="匹配模式" prop="pattern">
          <el-input
            v-model="formData.pattern"
            :placeholder="getPatternPlaceholder(formData.match_type)"
          />
        </el-form-item>
        <el-form-item label="匹配类型" prop="match_type">
          <el-select v-model="formData.match_type" placeholder="选择匹配类型">
            <el-option label="精确匹配" value="exact" />
            <el-option label="通配符" value="wildcard" />
            <el-option label="正则表达式" value="regex" />
          </el-select>
        </el-form-item>
        <el-form-item label="动作类型" prop="action_type">
          <el-select v-model="formData.action_type" placeholder="选择动作类型">
            <el-option label="映射到 IP" value="map_ip" />
            <el-option label="映射到域名" value="map_domain" />
            <el-option label="阻止" value="block" />
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="formData.action_type !== 'block'"
          label="动作值"
          prop="action_value"
        >
          <el-input
            v-model="formData.action_value"
            :placeholder="getActionValuePlaceholder(formData.action_type)"
          />
        </el-form-item>
        <el-form-item label="优先级" prop="priority">
          <el-input-number v-model="formData.priority" :min="0" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="formData.description"
            type="textarea"
            :rows="2"
            placeholder="规则描述（可选）"
          />
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

interface RewriteRule {
  id: number
  pattern: string
  match_type: string
  action_type: string
  action_value: string | null
  priority: number
  enabled: boolean
  description: string | null
  created_at: string
  updated_at: string
}

const rules = ref<RewriteRule[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const isEditing = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()
const editingId = ref<number | null>(null)

const formData = reactive({
  pattern: '',
  match_type: 'exact',
  action_type: 'block',
  action_value: '',
  priority: 0,
  description: '',
  enabled: true
})

const formRules: FormRules = {
  pattern: [
    { required: true, message: '请输入匹配模式', trigger: 'blur' },
    { max: 255, message: '匹配模式长度不能超过255个字符', trigger: 'blur' }
  ],
  match_type: [
    { required: true, message: '请选择匹配类型', trigger: 'change' }
  ],
  action_type: [
    { required: true, message: '请选择动作类型', trigger: 'change' }
  ]
}

function getMatchTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    exact: '精确',
    wildcard: '通配符',
    regex: '正则'
  }
  return labels[type] || type
}

function getMatchTypeTag(type: string): string {
  const tags: Record<string, string> = {
    exact: '',
    wildcard: 'warning',
    regex: 'danger'
  }
  return tags[type] || ''
}

function getActionTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    map_ip: '映射IP',
    map_domain: '映射域名',
    block: '阻止'
  }
  return labels[type] || type
}

function getActionTypeTag(type: string): string {
  const tags: Record<string, string> = {
    map_ip: 'success',
    map_domain: 'warning',
    block: 'danger'
  }
  return tags[type] || ''
}

function getPatternPlaceholder(matchType: string): string {
  const placeholders: Record<string, string> = {
    exact: 'ads.example.com',
    wildcard: '*.ads.com',
    regex: '^ads?\\.'
  }
  return placeholders[matchType] || ''
}

function getActionValuePlaceholder(actionType: string): string {
  const placeholders: Record<string, string> = {
    map_ip: '192.168.1.1 或 ::1',
    map_domain: 'target.example.com'
  }
  return placeholders[actionType] || ''
}

async function fetchRules() {
  loading.value = true
  try {
    const response = await api.get('/api/rewrite')
    rules.value = response.data.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.message || '获取规则失败')
  } finally {
    loading.value = false
  }
}

function resetForm() {
  formData.pattern = ''
  formData.match_type = 'exact'
  formData.action_type = 'block'
  formData.action_value = ''
  formData.priority = 0
  formData.description = ''
  formData.enabled = true
  editingId.value = null
}

function openCreateDialog() {
  isEditing.value = false
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(rule: RewriteRule) {
  isEditing.value = true
  editingId.value = rule.id
  formData.pattern = rule.pattern
  formData.match_type = rule.match_type
  formData.action_type = rule.action_type
  formData.action_value = rule.action_value || ''
  formData.priority = rule.priority
  formData.description = rule.description || ''
  formData.enabled = rule.enabled
  dialogVisible.value = true
}

async function submitForm() {
  if (!formRef.value) return
  
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    
    submitting.value = true
    try {
      const payload = {
        ...formData,
        action_value: formData.action_type === 'block' ? null : formData.action_value || null,
        description: formData.description || null
      }
      
      if (isEditing.value && editingId.value) {
        await api.put(`/api/rewrite/${editingId.value}`, payload)
        ElMessage.success('规则更新成功')
      } else {
        await api.post('/api/rewrite', payload)
        ElMessage.success('规则创建成功')
      }
      dialogVisible.value = false
      fetchRules()
    } catch (error: any) {
      const message = error.response?.data?.message || '操作失败'
      ElMessage.error(message)
    } finally {
      submitting.value = false
    }
  })
}

async function toggleEnabled(rule: RewriteRule) {
  try {
    await api.put(`/api/rewrite/${rule.id}`, { enabled: rule.enabled })
    ElMessage.success(rule.enabled ? '规则已启用' : '规则已禁用')
  } catch (error: any) {
    rule.enabled = !rule.enabled
    ElMessage.error(error.response?.data?.message || '操作失败')
  }
}

async function confirmDelete(rule: RewriteRule) {
  try {
    await ElMessageBox.confirm(
      `确定要删除规则 "${rule.pattern}" 吗？`,
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning'
      }
    )
    await api.delete(`/api/rewrite/${rule.id}`)
    ElMessage.success('规则删除成功')
    fetchRules()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchRules()
})
</script>

<style scoped>
.rewrite-rules {
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
