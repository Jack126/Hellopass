<template>
  <div class="add-password-form">
    <div class="form-group">
      <label for="site-name">网站名称 *</label>
      <input 
        type="text" 
        id="site-name"
        v-model="formData.site_name"
        placeholder="例如：公司内网"
        required
        @input="validateForm"
      />
    </div>
    
    <div class="form-group">
      <label for="site-url">网站/IP地址 *</label>
      <input 
        type="text" 
        id="site-url"
        v-model="formData.site_url"
        placeholder="例如：https://internal.company.com 或 192.168.1.100"
        required
        @input="validateForm"
      />
    </div>
    
    <div class="form-group">
      <label for="username">用户名/账号 *</label>
      <input 
        type="text" 
        id="username"
        v-model="formData.username"
        placeholder="例如：zhangsan@company.com"
        required
        @input="validateForm"
      />
    </div>
    
    <div class="form-group">
      <label for="password">密码 *</label>
      <div style="display: flex; gap: 10px;">
        <input 
          :type="showPassword ? 'text' : 'password'"
          id="password"
          v-model="formData.password"
          placeholder="输入密码"
          required
          @input="validateForm"
          style="flex: 1;"
        />
        <button type="button" @click="togglePasswordVisibility" class="btn btn-secondary btn-small">
          {{ showPassword ? '隐藏' : '显示' }}
        </button>
        <button type="button" @click="generatePassword" class="btn btn-secondary btn-small" title="生成强密码">
          生成
        </button>
      </div>
      
      <div class="password-strength">
        <div class="strength-bar">
          <div class="strength-fill" :class="passwordStrengthClass"></div>
        </div>
        <div class="strength-text">{{ passwordStrengthText }}</div>
      </div>
    </div>
    
    <div class="form-group">
      <label for="notes">备注（可选）</label>
      <textarea 
        id="notes"
        v-model="formData.notes"
        placeholder="添加备注信息..."
        rows="3"
        style="width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px;"
      />
    </div>
    
    <div class="form-group">
      <label>标签（可选）</label>
      <div style="display: flex; gap: 5px; flex-wrap: wrap; margin-bottom: 10px;">
        <input 
          type="text" 
          v-model="tagInput"
          placeholder="输入标签，按回车添加"
          @keyup.enter="addTag"
          style="flex: 1;"
        />
      </div>
      <div class="tags">
        <span v-for="tag in formData.tags" :key="tag" class="tag">
          {{ tag }}
          <button type="button" @click="removeTag(tag)" class="tag-remove">×</button>
        </span>
      </div>
    </div>
    
    <div class="form-group">
      <div class="checkbox-group">
        <input type="checkbox" v-model="formData.generatePassword" @change="handleGeneratePasswordChange">
        <label>自动生成密码</label>
      </div>
      
      <div v-if="formData.generatePassword" class="generate-options">
        <div class="form-group">
          <label for="password-length">密码长度</label>
          <select id="password-length" v-model="passwordOptions.length" @change="generatePassword">
            <option value="12">12 位</option>
            <option value="16">16 位</option>
            <option value="20">20 位</option>
            <option value="24">24 位</option>
          </select>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="passwordOptions.includeUppercase">
          <label>包含大写字母</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="passwordOptions.includeLowercase">
          <label>包含小写字母</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="passwordOptions.includeNumbers">
          <label>包含数字</label>
        </div>
        
        <div class="checkbox-group">
          <input type="checkbox" v-model="passwordOptions.includeSpecial">
          <label>包含特殊字符</label>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch } from 'vue'

const emit = defineEmits(['save', 'close'])

const props = defineProps({
  existingPassword: {
    type: Object,
    default: null
  }
})

// 响应式数据
const showPassword = ref(false)
const saving = ref(false)
const tagInput = ref('')
const formData = ref({
  site_name: '',
  site_url: '',
  username: '',
  password: '',
  notes: '',
  tags: [],
  generatePassword: false
})

const passwordOptions = ref({
  length: 16,
  includeUppercase: true,
  includeLowercase: true,
  includeNumbers: true,
  includeSpecial: true
})

// 计算属性
const isFormValid = computed(() => {
  return formData.value.site_name && 
         formData.value.site_url && 
         formData.value.username && 
         formData.value.password &&
         !saving.value
})

const passwordStrength = computed(() => {
  const password = formData.value.password
  let strength = 0
  
  // 长度评分
  strength += Math.min(password.length / 4, 25)
  
  // 复杂度评分
  if (password.match(/[a-z]/)) strength += 15
  if (password.match(/[A-Z]/)) strength += 15
  if (password.match(/[0-9]/)) strength += 15
  if (password.match(/[^a-zA-Z0-9]/)) strength += 25
  
  // 检查常见密码
  const commonPasswords = [
    'password', '123456', 'qwerty', 'letmein', 'welcome',
    'admin', 'password1', '12345678', 'abc123', 'password123'
  ]
  
  if (commonPasswords.includes(password.toLowerCase())) {
    strength = 0
  }
  
  return Math.min(strength, 100)
})

const passwordStrengthClass = computed(() => {
  if (passwordStrength.value < 30) return 'strength-weak'
  if (passwordStrength.value < 70) return 'strength-medium'
  return 'strength-strong'
})

const passwordStrengthText = computed(() => {
  if (passwordStrength.value < 30) return '弱'
  if (passwordStrength.value < 70) return '中'
  return '强'
})

// 方法
const togglePasswordVisibility = () => {
  showPassword.value = !showPassword.value
}

const validateForm = () => {
  if (formData.value.generatePassword) {
    generatePassword()
  }
}

const addTag = () => {
  if (tagInput.value.trim() && !formData.value.tags.includes(tagInput.value.trim())) {
    formData.value.tags.push(tagInput.value.trim())
    tagInput.value = ''
  }
}

const removeTag = (tag) => {
  formData.value.tags = formData.value.tags.filter(t => t !== tag)
}

const handleGeneratePasswordChange = () => {
  if (formData.value.generatePassword) {
    generatePassword()
  } else {
    formData.value.password = ''
  }
}

const generatePassword = () => {
  if (!formData.value.generatePassword) return
  
  try {
    const chars = {
      uppercase: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
      lowercase: 'abcdefghijklmnopqrstuvwxyz',
      numbers: '0123456789',
      special: '!@#$%^&*()_+-=[]{}|;:,.<>?'
    }
    
    let charset = ''
    if (passwordOptions.value.includeUppercase) charset += chars.uppercase
    if (passwordOptions.value.includeLowercase) charset += chars.lowercase
    if (passwordOptions.value.includeNumbers) charset += chars.numbers
    if (passwordOptions.value.includeSpecial) charset += chars.special
    
    if (charset === '') {
      alert('请至少选择一种字符类型')
      return
    }
    
    let password = ''
    const length = parseInt(passwordOptions.value.length)
    
    // 生成密码
    for (let i = 0; i < length; i++) {
      const randomIndex = Math.floor(Math.random() * charset.length)
      password += charset[randomIndex]
    }
    
    // 确保包含所有要求的字符类型
    if (passwordOptions.value.includeUppercase && !password.match(/[A-Z]/)) {
      password = password[0] + chars.uppercase[Math.floor(Math.random() * chars.uppercase.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeLowercase && !password.match(/[a-z]/)) {
      password = password[0] + chars.lowercase[Math.floor(Math.random() * chars.lowercase.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeNumbers && !password.match(/[0-9]/)) {
      password = password[0] + chars.numbers[Math.floor(Math.random() * chars.numbers.length)] + password.slice(1)
    }
    if (passwordOptions.value.includeSpecial && !password.match(/[^a-zA-Z0-9]/)) {
      password = password[0] + chars.special[Math.floor(Math.random() * chars.special.length)] + password.slice(1)
    }
    
    formData.value.password = password
  } catch (error) {
    console.error('生成密码失败:', error)
  }
}

const savePassword = async () => {
  if (!isFormValid.value) return
  
  saving.value = true
  
  try {
    // 这里应该调用实际的保存API
    // await axios.post('/api/add-password', formData.value)
    
    // 模拟保存延迟
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    emit('save', formData.value)
  } catch (error) {
    console.error('保存密码失败:', error)
    alert('保存密码失败')
  } finally {
    saving.value = false
  }
}

const closeForm = () => {
  emit('close')
}

const resetForm = () => {
  formData.value = {
    site_name: '',
    site_url: '',
    username: '',
    password: '',
    notes: '',
    tags: [],
    generatePassword: false
  }
  tagInput.value = ''
  passwordOptions.value = {
    length: 16,
    includeUppercase: true,
    includeLowercase: true,
    includeNumbers: true,
    includeSpecial: true
  }
  showPassword.value = false
  saving.value = false
}

// 监听密码输入变化
watch(() => formData.value.password, () => {
  // 密码强度计算已经在计算属性中处理
})

// 暴露方法给父组件
defineExpose({
  savePassword,
  closeForm,
  resetForm,
  generatePassword
})
</script>

<style scoped>
.add-password-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.form-group label {
  font-weight: 500;
  color: #333;
}

.form-group input,
.form-group select,
.form-group textarea {
  padding: 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 16px;
}

.form-group input:focus,
.form-group select:focus,
.form-group textarea:focus {
  outline: none;
  border-color: #3498db;
}

.form-group textarea {
  resize: vertical;
  min-height: 80px;
}

.password-input-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.password-strength {
  margin-top: 10px;
}

.strength-bar {
  height: 4px;
  background: #eee;
  border-radius: 2px;
  overflow: hidden;
}

.strength-fill {
  height: 100%;
  transition: all 0.3s;
}

.strength-weak {
  background: #e74c3c;
}

.strength-medium {
  background: #f39c12;
}

.strength-strong {
  background: #27ae60;
}

.strength-text {
  font-size: 12px;
  color: #666;
  margin-top: 5px;
}

.tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 5px;
}

.tag {
  background: #f8f9fa;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 12px;
  color: #666;
  display: flex;
  align-items: center;
  gap: 5px;
}

.tag-remove {
  background: none;
  border: none;
  cursor: pointer;
  color: #999;
  padding: 0;
  font-size: 12px;
}

.tag-remove:hover {
  color: #e74c3c;
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

.generate-options {
  margin-top: 15px;
  padding: 15px;
  background: #f8f9fa;
  border-radius: 4px;
}

.generate-options .form-group {
  margin-bottom: 10px;
}

.btn {
  padding: 10px 20px;
  border: none;
  border-radius: 4px;
  font-size: 16px;
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
  font-size: 14px;
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 768px) {
  .add-password-form {
    gap: 15px;
  }
  
  .form-group {
    gap: 3px;
  }
  
  .generate-options {
    padding: 10px;
  }
}
</style>