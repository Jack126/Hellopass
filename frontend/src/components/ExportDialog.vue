<template>
  <div class="modal" @click="closeDialog">
    <div class="modal-content" @click.stop>
      <div class="modal-header">
        <h3>导出密码</h3>
        <button @click="closeDialog" class="close-btn">
          <i class="fas fa-times"></i>
        </button>
      </div>
      
      <div class="export-body">
        <!-- 选择区域 -->
        <div class="selection-section">
          <div class="selection-header">
            <div class="selection-controls">
              <label class="select-all-label">
                <input 
                  type="checkbox" 
                  v-model="selectAll" 
                  @change="toggleSelectAll"
                />
                全选 ({{ filteredPasswords.length }} 条)
              </label>
              
              <button @click="clearSelection" class="btn btn-secondary btn-small">
                <i class="fas fa-times"></i>
                清除选择
              </button>
            </div>
            
            <div class="selection-info">
              <span class="selected-count">
                已选择 {{ selectedPasswords.size }} 条记录
              </span>
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
                <input 
                  type="checkbox" 
                  :value="password.id"
                  v-model="selectedPasswords"
                  @click.stop
                />
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
                
                <div class="password-meta">
                  <span class="created-date">
                    <i class="fas fa-calendar"></i>
                    {{ formatDate(password.created_at) }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 导出格式选择 -->
        <div class="format-section">
          <h4>导出格式</h4>
          <div class="format-options">
            <div class="format-option" @click="exportFormat = 'encrypted'">
              <div class="format-radio">
                <input 
                  type="radio" 
                  v-model="exportFormat" 
                  value="encrypted"
                />
                <div class="radio-custom"></div>
              </div>
              <div class="format-details">
                <div class="format-title">
                  <i class="fas fa-lock"></i>
                  加密格式
                </div>
                <div class="format-description">
                  密码已加密，可在其他程序中导入使用
                  <div class="format-security">
                    <i class="fas fa-shield-alt"></i>
                    需要主密码才能解密
                  </div>
                </div>
              </div>
            </div>
            
            <div class="format-option" @click="exportFormat = 'plaintext'">
              <div class="format-radio">
                <input 
                  type="radio" 
                  v-model="exportFormat" 
                  value="plaintext"
                />
                <div class="radio-custom"></div>
              </div>
              <div class="format-details">
                <div class="format-title">
                  <i class="fas fa-file-alt"></i>
                  明文格式
                </div>
                <div class="format-description">
                  密码为明文，仅用于备份
                  <div class="format-warning">
                    <i class="fas fa-exclamation-triangle"></i>
                    ⚠️ 不安全，请谨慎使用
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 导出选项 -->
        <div class="options-section">
          <h4>导出选项</h4>
          <div class="options-grid">
            <div class="option-group">
              <label>
                <input type="checkbox" v-model="exportOptions.includeMetadata">
                包含元数据
              </label>
              <small>创建时间、更新时间等信息</small>
            </div>
            
            <div class="option-group">
              <label>
                <input type="checkbox" v-model="exportOptions.includeTags">
                包含标签
              </label>
              <small>密码分类和标签信息</small>
            </div>
            
            <div class="option-group">
              <label>
                <input type="checkbox" v-model="exportOptions.includeNotes">
                包含备注
              </label>
              <small>密码备注信息</small>
            </div>
            
            <div class="option-group">
              <label>
                <input type="checkbox" v-model="exportOptions.compress">
                压缩文件
              </label>
              <small>减小导出文件大小</small>
            </div>
          </div>
        </div>
        
        <!-- 导出预览 -->
        <div class="preview-section" v-if="selectedPasswords.size > 0">
          <h4>导出预览</h4>
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
        <div class="security-notice" v-if="exportFormat === 'encrypted'">
          <div class="notice-icon">
            <i class="fas fa-info-circle"></i>
          </div>
          <div class="notice-content">
            <strong>安全提示：</strong>
            <ul>
              <li>加密导出文件需要主密码才能解密</li>
              <li>请妥善保管导出文件，不要泄露给他人</li>
              <li>建议通过安全通道传输导出文件</li>
              <li>定期备份重要数据</li>
            </ul>
          </div>
        </div>
        
        <div class="security-notice warning" v-else>
          <div class="notice-icon">
            <i class="fas fa-exclamation-triangle"></i>
          </div>
          <div class="notice-content">
            <strong>警告：</strong>
            明文格式会暴露所有密码信息，请仅在安全环境下使用，建议使用加密格式。
          </div>
        </div>
      </div>
      
      <div class="modal-footer">
        <button @click="closeDialog" class="btn btn-secondary">
          <i class="fas fa-times"></i>
          取消
        </button>
        
        <button 
          @click="exportPasswords" 
          class="btn btn-primary"
          :disabled="selectedPasswords.size === 0 || exporting"
        >
          <i v-if="exporting" class="fas fa-spinner fa-spin"></i>
          <i v-else class="fas fa-download"></i>
          {{ exporting ? '导出中...' : `导出 ${selectedPasswords.size} 条记录` }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'

const emit = defineEmits(['close', 'exported'])

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
const exporting = ref(false)
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
  
  exporting.value = true
  
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
    
    // 这里应该调用实际的导出API
    // const response = await axios.post('/api/export', exportData)
    
    // 模拟导出
    await new Promise(resolve => setTimeout(resolve, 2000))
    
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
    
    emit('exported', {
      count: selectedCount.value,
      format: exportFormat.value
    })
    
    closeDialog()
  } catch (error) {
    console.error('导出失败:', error)
    alert('导出失败，请重试')
  } finally {
    exporting.value = false
  }
}

const closeDialog = () => {
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
const updateSelectionInfo = () => {
  updateSelectAllState()
}

// 生命周期
onMounted(() => {
  // 监听键盘事件
  document.addEventListener('keydown', handleKeyDown)
})

// 清理
const handleKeyDown = (event) => {
  if (event.key === 'Escape') {
    closeDialog()
  }
  if (event.key === 'a' && event.ctrlKey) {
    event.preventDefault()
    toggleSelectAll()
  }
}

// 暴露方法给父组件
defineExpose({
  clearSelection,
  updateSelectionInfo
})
</script>

<style scoped>
.modal {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
}

.modal-content {
  background: white;
  border-radius: 12px;
  padding: 30px;
  max-width: 800px;
  width: 90%;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 25px;
  padding-bottom: 15px;
  border-bottom: 1px solid #e1e5e9;
}

.modal-header h3 {
  color: #333;
  font-size: 1.5rem;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  font-size: 1.5rem;
  cursor: pointer;
  color: #666;
  padding: 5px;
  border-radius: 50%;
  transition: all 0.3s;
}

.close-btn:hover {
  color: #333;
  background: #f8f9fa;
}

.export-body {
  display: flex;
  flex-direction: column;
  gap: 25px;
}

.selection-section {
  border: 1px solid #e1e5e9;
  border-radius: 8px;
  overflow: hidden;
}

.selection-header {
  background: #f8f9fa;
  padding: 15px 20px;
  border-bottom: 1px solid #e1e5e9;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.selection-controls {
  display: flex;
  align-items: center;
  gap: 15px;
}

.select-all-label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 500;
}

.select-all-label input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.btn-small {
  padding: 6px 12px;
  font-size: 14px;
}

.selection-info {
  color: #666;
  font-size: 14px;
}

.selected-count {
  font-weight: 600;
  color: #667eea;
}

.password-list {
  max-height: 300px;
  overflow-y: auto;
}

.password-item {
  display: flex;
  align-items: center;
  padding: 15px 20px;
  border-bottom: 1px solid #e1e5e9;
  cursor: pointer;
  transition: background-color 0.3s;
}

.password-item:hover {
  background: #f8f9fa;
}

.password-item.selected {
  background: #f0f4ff;
  border-left: 4px solid #667eea;
}

.selection-checkbox {
  margin-right: 15px;
}

.selection-checkbox input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.password-info {
  flex: 1;
}

.site-name {
  font-weight: 600;
  font-size: 1.1rem;
  margin-bottom: 5px;
  color: #333;
}

.site-url {
  color: #666;
  font-size: 0.9rem;
  margin-bottom: 3px;
}

.username {
  color: #888;
  font-size: 0.85rem;
  margin-bottom: 8px;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-bottom: 8px;
}

.tag {
  background: #e9ecef;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 0.8rem;
  color: #495057;
}

.password-meta {
  color: #999;
  font-size: 0.8rem;
}

.created-date {
  display: flex;
  align-items: center;
  gap: 5px;
}

.format-section,
.options-section,
.preview-section {
  border: 1px solid #e1e5e9;
  border-radius: 8px;
  padding: 20px;
}

.format-section h4,
.options-section h4,
.preview-section h4 {
  margin: 0 0 15px 0;
  color: #333;
  font-size: 1.1rem;
}

.format-options {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.format-option {
  display: flex;
  align-items: center;
  padding: 15px;
  border: 2px solid #e1e5e9;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s;
}

.format-option:hover {
  border-color: #667eea;
  background: #f8f9ff;
}

.format-option.selected {
  border-color: #667eea;
  background: #f0f4ff;
}

.format-radio {
  margin-right: 15px;
}

.format-radio input[type="radio"] {
  display: none;
}

.radio-custom {
  width: 20px;
  height: 20px;
  border: 2px solid #ddd;
  border-radius: 50%;
  position: relative;
  transition: all 0.3s;
}

.format-option.selected .radio-custom {
  border-color: #667eea;
  background: #667eea;
}

.format-option.selected .radio-custom::after {
  content: '';
  width: 8px;
  height: 8px;
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
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  margin-bottom: 8px;
  color: #333;
}

.format-description {
  color: #666;
  font-size: 0.9rem;
  line-height: 1.4;
}

.format-security,
.format-warning {
  display: flex;
  align-items: center;
  gap: 5px;
  margin-top: 8px;
  font-size: 0.85rem;
}

.format-security {
  color: #28a745;
}

.format-warning {
  color: #dc3545;
}

.options-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 20px;
}

.option-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.option-group label {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  font-weight: 500;
}

.option-group input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}

.option-group small {
  color: #666;
  font-size: 0.85rem;
}

.preview-info {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 15px;
}

.preview-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px;
  background: #f8f9fa;
  border-radius: 6px;
}

.preview-label {
  font-weight: 500;
  color: #666;
}

.preview-value {
  color: #333;
  font-weight: 600;
}

.security-notice {
  display: flex;
  gap: 15px;
  padding: 15px;
  border-radius: 8px;
  background: #e7f3ff;
  border: 1px solid #b8daff;
}

.security-notice.warning {
  background: #fff3cd;
  border: 1px solid #ffeaa7;
}

.notice-icon {
  color: #0066cc;
  font-size: 1.2rem;
  flex-shrink: 0;
}

.security-notice.warning .notice-icon {
  color: #856404;
}

.notice-content strong {
  color: #0066cc;
  margin-bottom: 8px;
  display: block;
}

.security-notice.warning .notice-content strong {
  color: #856404;
}

.notice-content ul {
  margin: 0;
  padding-left: 20px;
}

.notice-content li {
  margin-bottom: 5px;
  color: #555;
}

.security-notice.warning .notice-content li {
  color: #664d03;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 30px;
  padding-top: 20px;
  border-top: 1px solid #e1e5e9;
}

.btn {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s;
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn-primary {
  background: #667eea;
  color: white;
}

.btn-primary:hover {
  background: #5a6fd8;
}

.btn-secondary {
  background: #6c757d;
  color: white;
}

.btn-secondary:hover {
  background: #5a6268;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 768px) {
  .modal-content {
    width: 95%;
    margin: 20px;
    padding: 20px;
  }
  
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
  
  .modal-footer {
    flex-direction: column;
  }
  
  .btn {
    width: 100%;
    justify-content: center;
  }
}
</style>