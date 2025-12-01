--
-- Table structure for table `subscriptions`
--

DROP TABLE IF EXISTS subscriptions;
CREATE TABLE subscriptions (
  id bigserial PRIMARY KEY,
  subscription_id varchar(64) NOT NULL,
  user_id bigint NOT NULL,
  device_info varchar(255) DEFAULT NULL,
  created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  expires_at timestamptz DEFAULT NULL
);

-- 创建索引
CREATE UNIQUE INDEX subscriptions_subscription_id_idx ON subscriptions (subscription_id);
CREATE INDEX idx_subscriptions_subscription_id ON subscriptions (subscription_id);
CREATE INDEX idx_subscriptions_user_id ON subscriptions (user_id);
CREATE INDEX idx_subscriptions_expires_at ON subscriptions (expires_at);

-- 添加表注释
COMMENT ON TABLE subscriptions IS '订阅表';

-- 添加字段注释
COMMENT ON COLUMN subscriptions.id IS '主键ID';
COMMENT ON COLUMN subscriptions.subscription_id IS '订阅ID，格式：sub_{uuid}';
COMMENT ON COLUMN subscriptions.user_id IS '用户ID';
COMMENT ON COLUMN subscriptions.device_info IS '设备信息（可选）';
COMMENT ON COLUMN subscriptions.created_at IS '创建时间';
COMMENT ON COLUMN subscriptions.updated_at IS '更新时间';
COMMENT ON COLUMN subscriptions.expires_at IS '过期时间（可选，用于自动清理）';

--
-- Table structure for table `im_friendship`
--

DROP TABLE IF EXISTS im_friendship;
CREATE TABLE im_friendship (
  owner_id varchar(50) NOT NULL,
  to_id varchar(50) NOT NULL,
  remark varchar(50) DEFAULT NULL,
  del_flag integer DEFAULT NULL,
  black integer DEFAULT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  sequence bigint DEFAULT NULL,
  black_sequence bigint DEFAULT NULL,
  add_source varchar(20) DEFAULT NULL,
  extra varchar(1000) DEFAULT NULL,
  version bigint DEFAULT NULL,
  PRIMARY KEY(owner_id, to_id)
);

CREATE INDEX idx_im_friendship_owner_id ON im_friendship (owner_id);
CREATE INDEX idx_im_friendship_to_id ON im_friendship (to_id);

COMMENT ON TABLE im_friendship IS '好友关系表';
COMMENT ON COLUMN im_friendship.owner_id IS '用户ID';
COMMENT ON COLUMN im_friendship.to_id IS '好友用户ID';
COMMENT ON COLUMN im_friendship.remark IS '备注';
COMMENT ON COLUMN im_friendship.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_friendship.black IS '黑名单状态（1正常，2拉黑）';
COMMENT ON COLUMN im_friendship.create_time IS '创建时间';
COMMENT ON COLUMN im_friendship.update_time IS '更新时间';
COMMENT ON COLUMN im_friendship.sequence IS '序列号';
COMMENT ON COLUMN im_friendship.black_sequence IS '黑名单序列号';
COMMENT ON COLUMN im_friendship.add_source IS '好友来源';
COMMENT ON COLUMN im_friendship.extra IS '扩展字段';
COMMENT ON COLUMN im_friendship.version IS '版本信息';

--
-- Table structure for table `im_friendship_request`
--

DROP TABLE IF EXISTS im_friendship_request;
CREATE TABLE im_friendship_request (
  id varchar(50) NOT NULL PRIMARY KEY,
  from_id varchar(50) NOT NULL,
  to_id varchar(50) NOT NULL,
  remark varchar(50) DEFAULT NULL,
  read_status integer DEFAULT NULL,
  add_source varchar(20) DEFAULT NULL,
  message varchar(50) DEFAULT NULL,
  approve_status integer DEFAULT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  sequence bigint DEFAULT NULL,
  del_flag smallint DEFAULT NULL,
  version bigint DEFAULT NULL
);

CREATE INDEX idx_im_friendship_request_from_id ON im_friendship_request (from_id);
CREATE INDEX idx_im_friendship_request_to_id ON im_friendship_request (to_id);
CREATE INDEX idx_im_friendship_request_to_id_status ON im_friendship_request (to_id, approve_status);

COMMENT ON TABLE im_friendship_request IS '好友请求表';
COMMENT ON COLUMN im_friendship_request.id IS '请求ID';
COMMENT ON COLUMN im_friendship_request.from_id IS '请求发起者';
COMMENT ON COLUMN im_friendship_request.to_id IS '请求接收者';
COMMENT ON COLUMN im_friendship_request.remark IS '备注';
COMMENT ON COLUMN im_friendship_request.read_status IS '是否已读（1已读）';
COMMENT ON COLUMN im_friendship_request.add_source IS '好友来源';
COMMENT ON COLUMN im_friendship_request.message IS '好友验证信息';
COMMENT ON COLUMN im_friendship_request.approve_status IS '审批状态（1同意，2拒绝）';
COMMENT ON COLUMN im_friendship_request.create_time IS '创建时间';
COMMENT ON COLUMN im_friendship_request.update_time IS '更新时间';
COMMENT ON COLUMN im_friendship_request.sequence IS '序列号';
COMMENT ON COLUMN im_friendship_request.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_friendship_request.version IS '版本信息';

--
-- Table structure for table `im_user_data`
--

DROP TABLE IF EXISTS im_user_data;

CREATE TABLE im_user_data (
  user_id varchar(50) PRIMARY KEY,
  name varchar(100),
  avatar varchar(1024),
  gender integer,
  birthday varchar(50),
  location varchar(50),
  self_signature varchar(255),
  friend_allow_type integer NOT NULL,
  forbidden_flag integer NOT NULL,
  disable_add_friend integer NOT NULL,
  silent_flag integer NOT NULL,
  user_type integer NOT NULL,
  del_flag integer NOT NULL,
  extra varchar(1000),
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  version bigint
);

-- 添加表注释
COMMENT ON TABLE im_user_data IS '用户数据表';

-- 添加字段注释
COMMENT ON COLUMN im_user_data.user_id IS '用户ID';
COMMENT ON COLUMN im_user_data.name IS '昵称';
COMMENT ON COLUMN im_user_data.avatar IS '头像';
COMMENT ON COLUMN im_user_data.gender IS '性别';
COMMENT ON COLUMN im_user_data.birthday IS '生日';
COMMENT ON COLUMN im_user_data.location IS '地址';
COMMENT ON COLUMN im_user_data.self_signature IS '个性签名';
COMMENT ON COLUMN im_user_data.friend_allow_type IS '加好友验证类型（1无需验证，2需要验证）';
COMMENT ON COLUMN im_user_data.forbidden_flag IS '禁用标识（1禁用）';
COMMENT ON COLUMN im_user_data.disable_add_friend IS '管理员禁止添加好友：0未禁用，1已禁用';
COMMENT ON COLUMN im_user_data.silent_flag IS '禁言标识（1禁言）';
COMMENT ON COLUMN im_user_data.user_type IS '用户类型（1普通用户，2客服，3机器人）';
COMMENT ON COLUMN im_user_data.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_user_data.extra IS '扩展字段';
COMMENT ON COLUMN im_user_data.create_time IS '创建时间';
COMMENT ON COLUMN im_user_data.update_time IS '更新时间';
COMMENT ON COLUMN im_user_data.version IS '版本信息';

--
-- Table structure for table `users`
--
--
DROP TABLE IF EXISTS users;

CREATE TABLE users (
  id bigserial PRIMARY KEY,
  open_id varchar(32) NOT NULL,
  name varchar(100) NOT NULL,
  email varchar(255) NOT NULL,
  file_name varchar(256) DEFAULT NULL,
  abstract varchar(128) DEFAULT NULL,
  phone varchar(11) DEFAULT NULL,
  status integer DEFAULT 1,
  gender integer DEFAULT 3,
  password_hash varchar(255) NOT NULL,
  created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  version bigint DEFAULT 1,
  del_flag integer DEFAULT 1,
  create_time bigint DEFAULT NULL,
  update_time bigint DEFAULT NULL
);

-- 创建索引
CREATE UNIQUE INDEX users_email_idx ON users (email);
CREATE UNIQUE INDEX users_name_idx ON users (name);
CREATE UNIQUE INDEX users_open_id_idx ON users (open_id);
CREATE UNIQUE INDEX users_phone_idx ON users (phone);
CREATE INDEX users_status_idx ON users (status);

-- 添加表注释
COMMENT ON TABLE users IS '用户表';

-- 添加字段注释
COMMENT ON COLUMN users.id IS '主键ID';
COMMENT ON COLUMN users.open_id IS '外部唯一标识符（雪花算法生成的数字字符串，最多20字符）';
COMMENT ON COLUMN users.name IS '用户名';
COMMENT ON COLUMN users.email IS '邮箱';
COMMENT ON COLUMN users.file_name IS '头像文件名';
COMMENT ON COLUMN users.abstract IS '个性签名';
COMMENT ON COLUMN users.phone IS '手机号';
COMMENT ON COLUMN users.status IS '状态：1正常 2禁用 3删除';
COMMENT ON COLUMN users.gender IS '性别：1男 2女 3未知';
COMMENT ON COLUMN users.password_hash IS '密码哈希';
COMMENT ON COLUMN users.created_at IS '创建时间';
COMMENT ON COLUMN users.updated_at IS '更新时间';
COMMENT ON COLUMN users.version IS '版本号';
COMMENT ON COLUMN users.del_flag IS '删除标志：1=正常，0=删除';
