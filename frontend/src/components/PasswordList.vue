<template>
  <div class="password-list">
    <div v-if="loading" class="loading">
      <span class="spinner"></span>
      <span>加载中...</span>
    </div>
    
    <div v-else-if="filteredPasswords.length === 0" class="empty-state">
      <div>🔒</div>
      <p>暂无密码记录</p>
      <p>点击"添加密码"开始使用</p>
    </div>
    
    <div v-else>
      <div 
        v-for="password in filteredPasswords" 
        :key="password.id"
        class="password-item"
        :class="{ selected: selectedPasswords.has(password.id) }"
        @click="togglePasswordSelection(password.id)"
      >
        <div class="site-icon">
          {{ getSiteInitial(password.site_name) }}
        </div>
        
        <div class="password-info">
          <div class="site-name">{{ password.site_name }}</div>
          <div class="site-url">{{ password.site_url }}</div>
          <div class="username">{{ password.username }}</div>
          
          <div v-if="password.tags && password.tags.length > 0" class="tags">
            <span v-for="tag in password.get_tags()" :key="tag" class="tag">
              {{ tag }}
              <button type="button" @click.stop="removeTag(password, tag)" class="tag-remove">×</button>
            </span>
          </div>
        </div>
        
        <div class="password-actions">
          <button 
            @click.stop="copyPassword(password)" 
            class="btn-icon btn-success"
            title="复制密码"
          >
            复制
          </button>
          
          <button 
            @click.stop="editPassword(password)" 
            class="btn-icon btn-secondary"
            title="编辑"
          >
            编辑
          </button>
          
          <button 
            @click.stop="deletePassword(password)" 
            class="btn-icon btn-danger"
            title="删除"
          >
            删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const emit = defineEmits(['select', 'copy', 'edit', 'delete'])

const props = defineProps({
  passwords: {
    type: Array,
    default: () => []
  },
  selectedPasswords: {
    type: Set,
    default: () => new Set()
  },
  loading: {
    type: Boolean,
    default: false
  },
  searchQuery: {
    type: String,
    default: ''
  }
})

// 计算属性
const filteredPasswords = computed(() => {
  if (!props.searchQuery) return props.passwords
  
  const query = props.searchQuery.toLowerCase()
  return props.passwords.filter(password => 
    password.site_name.toLowerCase().includes(query) ||
    password.site_url.toLowerCase().includes(query) ||
    password.username.toLowerCase().includes(query)
  )
})

// 方法
const togglePasswordSelection = (id) => {
  emit('select', id)
}

const copyPassword = (password) => {
  emit('copy', password)
}

const editPassword = (password) => {
  emit('edit', password)
}

const deletePassword = (password) => {
  emit('delete', password)
}

const removeTag = (password, tag) => {
  if (password.tags) {
    password.tags = password.tags.filter(t => t !== tag)
  }
}

const getSiteInitial = (siteName) => {
  return siteName.charAt(0).toUpperCase()
}
</script>

<style scoped>
.password-list {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 10px rgba(0,0,0,0.1);
}

.loading {
  text-align: center;
  padding: 40px;
  color: #666;
}

.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid #f3f3f3;
  border-top: 2px solid #3498db;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-right: 10px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #666;
}

.empty-state div {
  font-size: 48px;
  margin-bottom: 20px;
}

.password-item {
  display: flex;
  align-items: center;
  padding: 15px;
  border: 1px solid #eee;
  border-radius: 4px;
  margin-bottom: 10px;
  transition: background-color 0.3s;
  cursor: pointer;
}

.password-item:hover {
  background: #f8f9fa;
}

.password-item.selected {
  background: #e3f2fd;
  border-color: #3498db;
}

.site-icon {
  width: 40px;
  height: 40px;
  border-radius: 4px;
  margin-right: 15px;
  background: #3498db;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: bold;
  font-size: 16px;
}

.password-info {
  flex: 1;
}

.site-name {
  font-weight: 600;
  font-size: 16px;
  margin-bottom: 3px;
}

.site-url {
  color: #666;
  font-size: 14px;
  margin-bottom: 3px;
}

.username {
  color: #888;
  font-size: 14px;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 8px;
}

.tag {
  background: #f8f9fa;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  color: #666;
}

.tag-remove {
  background: none;
  border: none;
  cursor: pointer;
  color: #999;
  padding: 0;
  margin-left: 5px;
  font-size: 12px;
}

.tag-remove:hover {
  color: #e74c3c;
}

.password-actions {
  display: flex;
  gap: 5px;
}

.btn-icon {
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  border: none;
  font-size: 14px;
  transition: background-color 0.3s;
}

.btn-success {
  background: #27ae60;
  color: white;
}

.btn-success:hover {
  background: #229954;
}

.btn-secondary {
  background: #95a5a6;
  color: white;
}

.btn-secondary:hover {
  background: #7f8c8d;
}

.btn-danger {
  background: #e74c3c;
  color: white;
}

.btn-danger:hover {
  background: #c0392b;
}

@media (max-width: 768px) {
  .password-item {
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }
  
  .password-actions {
    align-self: flex-end;
  }
}
</style>