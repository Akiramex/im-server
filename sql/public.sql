--
-- Table structure for table im_group_message
--

DROP TABLE IF EXISTS im_group_message;
CREATE TABLE im_group_message (
  message_id varchar(512) NOT NULL,
  group_id varchar(255) NOT NULL,
  from_id varchar(20) NOT NULL,
  message_body text NOT NULL,
  message_time timestamptz NOT NULL,
  message_content_type integer NOT NULL,
  extra text DEFAULT NULL,
  del_flag smallint NOT NULL,
  sequence bigint DEFAULT NULL,
  message_random varchar(255) DEFAULT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  version bigint DEFAULT NULL,
  reply_to varchar(255) DEFAULT NULL,
  PRIMARY KEY (message_id)
);

-- 创建索引
CREATE INDEX idx_group_msg_group ON im_group_message (group_id);
CREATE INDEX idx_from_id ON im_group_message (from_id);
CREATE INDEX idx_group_msg_sequence ON im_group_message (sequence);

-- 添加表注释
COMMENT ON TABLE im_group_message IS '群聊消息表';

-- 添加字段注释
COMMENT ON COLUMN im_group_message.message_id IS '消息ID';
COMMENT ON COLUMN im_group_message.group_id IS '群组ID';
COMMENT ON COLUMN im_group_message.from_id IS '发送者用户ID';
COMMENT ON COLUMN im_group_message.message_body IS '消息内容';
COMMENT ON COLUMN im_group_message.message_time IS '发送时间';
COMMENT ON COLUMN im_group_message.message_content_type IS '消息类型';
COMMENT ON COLUMN im_group_message.extra IS '扩展字段';
COMMENT ON COLUMN im_group_message.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_group_message.sequence IS '消息序列';
COMMENT ON COLUMN im_group_message.message_random IS '随机标识';
COMMENT ON COLUMN im_group_message.create_time IS '创建时间';
COMMENT ON COLUMN im_group_message.update_time IS '更新时间';
COMMENT ON COLUMN im_group_message.version IS '版本信息';
COMMENT ON COLUMN im_group_message.reply_to IS '被引用的消息 ID';


--
-- Table structure for table im_single_message
--

DROP TABLE IF EXISTS im_single_message;
CREATE TABLE im_single_message (
  message_id varchar(512) NOT NULL,
  from_id varchar(50) NOT NULL,
  to_id varchar(50) NOT NULL,
  message_body text NOT NULL,
  message_time timestamptz NOT NULL,
  message_content_type integer NOT NULL,
  read_status integer NOT NULL,
  extra text DEFAULT NULL,
  del_flag smallint NOT NULL,
  sequence bigint NOT NULL,
  message_random varchar(255) DEFAULT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  version bigint DEFAULT NULL,
  reply_to varchar(255) DEFAULT NULL,
  to_type text DEFAULT 'User',
  file_url varchar(512) DEFAULT NULL,
  file_name varchar(255) DEFAULT NULL,
  file_type varchar(64) DEFAULT NULL,
  PRIMARY KEY (message_id)
);

-- 创建索引
CREATE INDEX idx_private_from ON im_single_message (from_id);
CREATE INDEX idx_private_to ON im_single_message (to_id);
CREATE INDEX idx_single_msg_sequence ON im_single_message (sequence);

-- 添加表注释
COMMENT ON TABLE im_single_message IS '单聊消息表';

-- 添加字段注释
COMMENT ON COLUMN im_single_message.message_id IS '消息ID';
COMMENT ON COLUMN im_single_message.from_id IS '发送者用户ID';
COMMENT ON COLUMN im_single_message.to_id IS '接收者用户ID';
COMMENT ON COLUMN im_single_message.message_body IS '消息内容';
COMMENT ON COLUMN im_single_message.message_time IS '发送时间';
COMMENT ON COLUMN im_single_message.message_content_type IS '消息类型';
COMMENT ON COLUMN im_single_message.read_status IS '阅读状态（1已读）';
COMMENT ON COLUMN im_single_message.extra IS '扩展字段';
COMMENT ON COLUMN im_single_message.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_single_message.sequence IS '消息序列';
COMMENT ON COLUMN im_single_message.message_random IS '随机标识';
COMMENT ON COLUMN im_single_message.create_time IS '创建时间';
COMMENT ON COLUMN im_single_message.update_time IS '更新时间';
COMMENT ON COLUMN im_single_message.version IS '版本信息';
COMMENT ON COLUMN im_single_message.reply_to IS '被引用的消息 ID';
COMMENT ON COLUMN im_single_message.to_type IS '接收者类型：User=用户，Group=群组';
COMMENT ON COLUMN im_single_message.file_url IS '文件URL';
COMMENT ON COLUMN im_single_message.file_name IS '文件名';
COMMENT ON COLUMN im_single_message.file_type IS '文件类型';

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

--
-- Table structure for table im_group
--

DROP TABLE IF EXISTS im_group;
CREATE TABLE im_group (
  group_id varchar(50) NOT NULL,
  owner_id varchar(50) NOT NULL,
  group_type integer NOT NULL,
  group_name varchar(100) NOT NULL,
  mute smallint DEFAULT NULL,
  apply_join_type integer NOT NULL,
  avatar varchar(300) DEFAULT NULL,
  max_member_count integer DEFAULT NULL,
  introduction varchar(100) DEFAULT NULL,
  notification varchar(1000) DEFAULT NULL,
  status integer DEFAULT NULL,
  sequence bigint DEFAULT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  extra varchar(1000) DEFAULT NULL,
  version bigint DEFAULT NULL,
  del_flag smallint NOT NULL,
  verifier smallint DEFAULT NULL,
  PRIMARY KEY (group_id)
);

-- 创建索引
CREATE INDEX idx_owner_id ON im_group (owner_id);
CREATE INDEX idx_status ON im_group (status);

-- 添加表注释
COMMENT ON TABLE im_group IS '群组表';

-- 添加字段注释
COMMENT ON COLUMN im_group.group_id IS '群组ID';
COMMENT ON COLUMN im_group.owner_id IS '群主用户ID';
COMMENT ON COLUMN im_group.group_type IS '群类型（1私有群，2公开群）';
COMMENT ON COLUMN im_group.group_name IS '群名称';
COMMENT ON COLUMN im_group.mute IS '是否全员禁言（1不禁言，0禁言）';
COMMENT ON COLUMN im_group.apply_join_type IS '申请加群方式（0禁止申请，1需要审批，2允许自由加入）';
COMMENT ON COLUMN im_group.avatar IS '群头像';
COMMENT ON COLUMN im_group.max_member_count IS '最大成员数';
COMMENT ON COLUMN im_group.introduction IS '群简介';
COMMENT ON COLUMN im_group.notification IS '群公告';
COMMENT ON COLUMN im_group.status IS '群状态（1正常，0解散）';
COMMENT ON COLUMN im_group.sequence IS '消息序列号';
COMMENT ON COLUMN im_group.create_time IS '创建时间';
COMMENT ON COLUMN im_group.update_time IS '更新时间';
COMMENT ON COLUMN im_group.extra IS '扩展字段';
COMMENT ON COLUMN im_group.version IS '版本信息';
COMMENT ON COLUMN im_group.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_group.verifier IS '开启群验证（1验证，0不验证）';

--
-- Table structure for table im_group_member
--

DROP TABLE IF EXISTS im_group_member;
CREATE TABLE im_group_member (
  group_member_id varchar(100) NOT NULL,
  group_id varchar(50) NOT NULL,
  member_id varchar(50) NOT NULL,
  role integer NOT NULL,
  speak_date timestamptz DEFAULT NULL,
  mute smallint NOT NULL,
  alias varchar(100) DEFAULT NULL,
  join_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  leave_time timestamptz DEFAULT NULL,
  join_type varchar(50) DEFAULT NULL,
  extra varchar(1000) DEFAULT NULL,
  del_flag smallint NOT NULL,
  create_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  update_time timestamptz DEFAULT CURRENT_TIMESTAMP,
  version bigint DEFAULT NULL,
  PRIMARY KEY (group_member_id)
);

-- 创建索引
CREATE INDEX idx_group_id ON im_group_member (group_id);
CREATE INDEX idx_igm_member_group ON im_group_member (member_id, group_id);
CREATE INDEX idx_member_id ON im_group_member (member_id);

-- 添加表注释
COMMENT ON TABLE im_group_member IS '群组成员表';

-- 添加字段注释
COMMENT ON COLUMN im_group_member.group_member_id IS '群组成员ID';
COMMENT ON COLUMN im_group_member.group_id IS '群组ID';
COMMENT ON COLUMN im_group_member.member_id IS '成员用户ID';
COMMENT ON COLUMN im_group_member.role IS '群成员角色（0普通成员，1管理员，2群主）';
COMMENT ON COLUMN im_group_member.speak_date IS '最后发言时间';
COMMENT ON COLUMN im_group_member.mute IS '是否禁言（1不禁言，0禁言）';
COMMENT ON COLUMN im_group_member.alias IS '群昵称';
COMMENT ON COLUMN im_group_member.join_time IS '加入时间';
COMMENT ON COLUMN im_group_member.leave_time IS '离开时间';
COMMENT ON COLUMN im_group_member.join_type IS '加入类型';
COMMENT ON COLUMN im_group_member.extra IS '扩展字段';
COMMENT ON COLUMN im_group_member.del_flag IS '删除标识（1正常，0删除）';
COMMENT ON COLUMN im_group_member.create_time IS '创建时间';
COMMENT ON COLUMN im_group_member.update_time IS '更新时间';
COMMENT ON COLUMN im_group_member.version IS '版本信息';

--
-- Table structure for table im_outbox
--

DROP TABLE IF EXISTS im_outbox;
CREATE TABLE im_outbox (
  id bigserial NOT NULL,
  message_id varchar(64) NOT NULL,
  payload text NOT NULL,
  exchange varchar(128) NOT NULL,
  routing_key varchar(128) NOT NULL,
  attempts integer NOT NULL DEFAULT 0,
  status varchar(20) NOT NULL DEFAULT 'PENDING',
  last_error text DEFAULT NULL,
  created_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  updated_at timestamptz DEFAULT CURRENT_TIMESTAMP,
  next_try_at timestamptz DEFAULT NULL,
  PRIMARY KEY (id)
);

-- 创建索引
CREATE INDEX idx_outbox_message_id ON im_outbox (message_id);
CREATE INDEX idx_outbox_status ON im_outbox (status);

-- 添加表注释
COMMENT ON TABLE im_outbox IS 'Outbox table: 持久化要投递到 MQ 的消息，支持重试/幂等/确认回写';

-- 添加字段注释
COMMENT ON COLUMN im_outbox.id IS '主键';
COMMENT ON COLUMN im_outbox.message_id IS '业务消息 ID（用于回溯/去重/关联业务数据）';
COMMENT ON COLUMN im_outbox.payload IS '要发送的 JSON 负载（建议尽量轻量：可仅包含 messageId + 必要路由信息）';
COMMENT ON COLUMN im_outbox.exchange IS '目标交换机名称';
COMMENT ON COLUMN im_outbox.routing_key IS '目标路由键（或 queue 名称）';
COMMENT ON COLUMN im_outbox.attempts IS '累积投递次数';
COMMENT ON COLUMN im_outbox.status IS '投递状态：PENDING(待投递) / SENT(已确认) / FAILED(失败，需要人工介入) / DLX(死信)';
COMMENT ON COLUMN im_outbox.last_error IS '投递失败时的错误信息';
COMMENT ON COLUMN im_outbox.created_at IS '创建时间';
COMMENT ON COLUMN im_outbox.updated_at IS '更新时间';
COMMENT ON COLUMN im_outbox.next_try_at IS '下一次重试时间（用以调度延迟重试）';
