<template>
  <div class="export-form">
    <!-- 选择区域 -->
    <div class="selection-section">
      <div class="selection-header">
        <div class="selection-controls">
          <div class="checkbox-group">
            <input type="checkbox" v-model="selectAll" @change="toggleSelectAll">
            <label>全选 ({{ filteredPasswords.length }} 条)</label>
          </div>
          <button @click="clearSelection" class="btn btn-secondary btn-small">清除选择</button>
        </div>
        
        <div class="selection-info">
          <span class="selected-count">已选择 {{ selectedPasswords.size }} 条记录</span>
        </div>
      </div>
      
      <div class="password-list">
        <div 
          v-for="password in filteredPasswords" 
          :key="password.id"
          class="password-item"
          :class="{ selected: selectedPasswords.has(password.id) }"
          @click="togglePasswordSelection(password.id)"
        >
          <div class="selection-checkbox">
            <input type="checkbox" :value="password.id" v-model="selectedPasswords" @click.stop>
          </div>
          
          <div class="password-info">
            <div class="site-name">{{ password.site_name }}</div>
            <div class="site-url">{{ password.site_url }}</div>
            <div class="username">{{ password.username }}</div>
            
            <div v-if="password.tags && password.tags.length > 0" class="tags">
              <span v-for="tag in password.get_tags()" :key="tag" class="tag">
                {{ tag }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 导出格式选择 -->
    <div class="format-section">
      <label>导出格式</label>
      <div class="format-options">
        <div class="format-option" @click="exportFormat = 'encrypted'" :class="{ selected: exportFormat === 'encrypted' }">
          <div class="format-radio">
            <input type="radio" v-model="exportFormat" value="encrypted">
            <div class="radio-custom"></div>
          </div>
          <div class="format-details">
            <div class="format-title">加密格式</div>
            <div class="format-description">密码已加密，可在其他程序中导入使用</div>
          </div>
        </div>
        
        <div class="format-option" @click="exportFormat = 'plaintext'" :class="{ selected: exportFormat === 'plaintext' }">
          <div class="format-radio">
            <input type="radio" v-model="exportFormat" value="plaintext">
            <div class="radio-custom"></div>
          </div>
          <div class="format-details">
            <div class="format-title">明文格式</div>
            <div class="format-description">密码为明文，仅用于备份（不推荐）</div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 导出选项 -->
    <div class="options-section">
      <label>导出选项</label>
      <div class="options-grid">
        <div class="checkbox-group">
          <input type="checkbox" v-model="exportOptions.includeMetadata">
          <label>包含元数据</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="exportOptions.includeTags">
          <label>包含标签</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="exportOptions.includeNotes">
          <label>包含备注</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="exportOptions.compress">
          <label>压缩文件</label>
        </div>
      </div>
    </div>
    
    <!-- 导出预览 -->
    <div v-if="selectedPasswords.size > 0" class="preview-section">
      <label>导出预览</label>
      <div class="preview-info">
        <div class="preview-item">
          <span class="preview-label">导出格式:</span>
          <span class="preview-value">{{ exportFormat === 'encrypted' ? '加密' : '明文' }}</span>
        </div>
        <div class="preview-item">
          <span class="preview-label">记录数量:</span>
          <span class="preview-value">{{ selectedPasswords.size }} 条</span>
        </div>
        <div class="preview-item">
          <span class="preview-label">文件大小:</span>
          <span class="preview-value">{{ estimatedFileSize }}</span>
        </div>
        <div class="preview-item">
          <span class="preview-label">导出时间:</span>
          <span class="preview-value">{{ currentDateTime }}</span>
        </div>
      </div>
    </div>
    
    <!-- 安全提醒 -->
    <div v-if="selectedPasswords.size > 0" class="security-notice" :class="{ warning: exportFormat === 'plaintext' }">
      <div v-if="exportFormat === 'encrypted'">
        <strong>安全提示：</strong>
        <ul>
          <li>加密导出文件需要主密码才能解密</li>
          <li>请妥善保管导出文件，不要泄露给他人</li>
          <li>建议通过安全通道传输导出文件</li>
        </ul>
      </div>
      <div v-else>
        <strong>警告：</strong>
        明文格式会暴露所有密码信息，请仅在安全环境下使用，建议使用加密格式。
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'

const emit = defineEmits(['export', 'close'])

const props = defineProps({
  passwords: {
    type: Array,
    default: () => []
  }
})

// 响应式数据
const selectedPasswords = ref(new Set())
const selectAll = ref(false)
const exportFormat = ref('encrypted')
const exportOptions = ref({
  includeMetadata: true,
  includeTags: true,
  includeNotes: false,
  compress: false
})

// 计算属性
const filteredPasswords = computed(() => {
  return props.passwords
})

const selectedCount = computed(() => {
  return selectedPasswords.value.size
})

const estimatedFileSize = computed(() => {
  const baseSize = selectedCount.value * 200 // 每条记录约200字节
  const metadataSize = exportOptions.value.includeMetadata ? baseSize * 0.2 : 0
  const tagsSize = exportOptions.value.includeTags ? baseSize * 0.1 : 0
  const notesSize = exportOptions.value.includeNotes ? baseSize * 0.3 : 0
  const totalSize = baseSize + metadataSize + tagsSize + notesSize
  
  if (exportOptions.value.compress) {
    return formatFileSize(totalSize * 0.3) // 压缩后约30%
  }
  
  return formatFileSize(totalSize)
})

const currentDateTime = computed(() => {
  return new Date().toLocaleString('zh-CN')
})

// 方法
const togglePasswordSelection = (id) => {
  if (selectedPasswords.value.has(id)) {
    selectedPasswords.value.delete(id)
  } else {
    selectedPasswords.value.add(id)
  }
  
  updateSelectAllState()
}

const toggleSelectAll = () => {
  if (selectAll.value) {
    filteredPasswords.value.forEach(password => {
      selectedPasswords.value.add(password.id)
    })
  } else {
    selectedPasswords.value.clear()
  }
}

const updateSelectAllState = () => {
  selectAll.value = selectedPasswords.value.size === filteredPasswords.value.length
}

const clearSelection = () => {
  selectedPasswords.value.clear()
  selectAll.value = false
}

const exportPasswords = async () => {
  if (selectedPasswords.value.size === 0) {
    alert('请选择要导出的记录')
    return
  }
  
  try {
    // 准备导出数据
    const selectedIds = Array.from(selectedPasswords.value)
    const passwordsToExport = filteredPasswords.value.filter(password => 
      selectedIds.includes(password.id)
    )
    
    const exportData = {
      version: '1.0',
      timestamp: new Date().toISOString(),
      format: exportFormat.value,
      options: exportOptions.value,
      count: passwordsToExport.length,
      passwords: passwordsToExport
    }
    
    // 创建下载文件
    const dataStr = JSON.stringify(exportData, null, 2)
    const blob = new Blob([dataStr], { 
      type: 'application/json' 
    })
    
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    
    const timestamp = new Date().toISOString().split('T')[0]
    const formatExt = exportFormat.value === 'encrypted' ? 'enc' : 'json'
    a.download = `securepass_export_${timestamp}.${formatExt}`
    
    a.click()
    URL.revokeObjectURL(url)
    
    emit('export', {
      count: selectedCount.value,
      format: exportFormat.value
    })
  } catch (error) {
    console.error('导出失败:', error)
    alert('导出失败，请重试')
  }
}

const closeForm = () => {
  emit('close')
}

const formatDate = (dateString) => {
  try {
    const date = new Date(dateString)
    return date.toLocaleDateString('zh-CN')
  } catch (error) {
    return dateString
  }
}

const formatFileSize = (bytes) => {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
  return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB'
}

// 监听选择变化
watch(() => selectedPasswords.value.size, () => {
  updateSelectAllState()
})

// 暴露方法给父组件
defineExpose({
  exportPasswords,
  closeForm,
  clearSelection
})
</script>

<style scoped>
.export-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.selection-section,
.format-section,
.options-section,
.preview-section {
  border: 1px solid #ddd;
  border-radius: 4px;
  padding: 15px;
}

.selection-section {
  border: 1px solid #ddd;
  border-radius: 4px;
  overflow: hidden;
}

.selection-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding-bottom: 10px;
  border-bottom: 1px solid #eee;
}

.selection-controls {
  display: flex;
  align-items: center;
  gap: 15px;
}

.checkbox-group {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.checkbox-group input[type="checkbox"] {
  width: 16px;
  height: 16px;
}

.selected-count {
  font-weight: 600;
  color: #3498db;
}

.password-list {
  max-height: 300px;
  overflow-y: auto;
  border: 1px solid #eee;
  border-radius: 4px;
}

.password-item {
  display: flex;
  align-items: center;
  padding: 12px;
  border-bottom: 1px solid #eee;
  cursor: pointer;
  transition: background-color 0.3s;
}

.password-item:hover {
  background: #f8f9fa;
}

.password-item.selected {
  background: #e3f2fd;
  border-left: 3px solid #3498db;
}

.selection-checkbox {
  margin-right: 12px;
}

.password-info {
  flex: 1;
}

.site-name {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 3px;
}

.site-url {
  color: #666;
  font-size: 12px;
  margin-bottom: 3px;
}

.username {
  color: #888;
  font-size: 12px;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 3px;
  margin-top: 5px;
}

.tag {
  background: #f8f9fa;
  padding: 2px 6px;
  border-radius: 8px;
  font-size: 11px;
  color: #666;
}

.format-section label,
.options-section label,
.preview-section label {
  font-weight: 500;
  margin-bottom: 10px;
  display: block;
}

.format-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.format-option {
  border: 1px solid #ddd;
  border-radius: 4px;
  padding: 12px;
  cursor: pointer;
  transition: background-color 0.3s;
}

.format-option:hover {
  background: #f8f9fa;
}

.format-option.selected {
  background: #e3f2fd;
  border-color: #3498db;
}

.format-radio {
  margin-right: 12px;
}

.format-radio input[type="radio"] {
  display: none;
}

.radio-custom {
  width: 16px;
  height: 16px;
  border: 2px solid #ddd;
  border-radius: 50%;
  position: relative;
  transition: all 0.3s;
}

.format-option.selected .radio-custom {
  border-color: #3498db;
  background: #3498db;
}

.format-option.selected .radio-custom::after {
  content: '';
  width: 6px;
  height: 6px;
  background: white;
  border-radius: 50%;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.format-details {
  flex: 1;
}

.format-title {
  font-weight: 600;
  margin-bottom: 5px;
  font-size: 14px;
}

.format-description {
  font-size: 12px;
  color: #666;
  line-height: 1.4;
}

.options-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 15px;
}

.preview-info {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
}

.preview-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  background: #f8f9fa;
  border-radius: 4px;
}

.preview-label {
  font-weight: 500;
  color: #666;
  font-size: 12px;
}

.preview-value {
  color: #333;
  font-weight: 600;
  font-size: 12px;
}

.security-notice {
  background: #e7f3ff;
  border: 1px solid #b8daff;
  border-radius: 4px;
  padding: 15px;
}

.security-notice.warning {
  background: #fff3cd;
  border: 1px solid #ffeaa7;
}

.security-notice strong {
  color: #0066cc;
  margin-bottom: 8px;
  display: block;
}

.security-notice.warning strong {
  color: #856404;
}

.security-notice ul {
  margin: 0;
  padding-left: 20px;
}

.security-notice li {
  margin-bottom: 5px;
  color: #555;
  font-size: 14px;
}

.security-notice.warning li {
  color: #664d03;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.3s;
}

.btn-primary {
  background: #3498db;
  color: white;
}

.btn-primary:hover {
  background: #2980b9;
}

.btn-secondary {
  background: #95a5a6;
  color: white;
}

.btn-secondary:hover {
  background: #7f8c8d;
}

.btn-small {
  padding: 6px 12px;
  font-size: 12px;
}

@media (max-width: 768px) {
  .selection-header {
    flex-direction: column;
    gap: 10px;
    align-items: flex-start;
  }
  
  .options-grid {
    grid-template-columns: 1fr;
  }
  
  .preview-info {
    grid-template-columns: 1fr;
  }
}
</style>