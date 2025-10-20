import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogOverlay,
  Badge,
  Box,
  Button,
  Container,
  Divider,
  Flex,
  FormControl,
  FormLabel,
  Heading,
  HStack,
  IconButton,
  Input,
  Link,
  NumberInput,
  NumberInputField,
  Select,
  Spacer,
  Stack,
  Switch,
  Text,
  VStack,
  useDisclosure,
  useToast,
} from "@chakra-ui/react";
import { CopyIcon, DeleteIcon, RepeatIcon } from "@chakra-ui/icons";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import axios from "axios";
import type {
  Bucket,
  CreateLinkRequest,
  CreateLinkResponse,
  DownloadLinkResponse,
  ListBucketsResponse,
  ListLinksResponse,
  ListObjectsResponse,
  LoginResponse,
  ObjectInfo,
  UserInfoResponse,
} from "./types";
import { API_CONFIG, OAUTH_CONFIG } from "./config";
import {
  buildAuthorizeUrl,
  generateCodeChallenge,
  generateCodeVerifier,
  generateState,
  getAndClearOAuthSession,
  storeOAuthSession,
} from "./oauth";

const TOKEN_STORAGE_KEY = "signed-download-token";
const TOKEN_EXPIRY_STORAGE_KEY = "signed-download-token-exp";
const USERNAME_STORAGE_KEY = "signed-download-username";

interface LinkFormState {
  bucket: string;
  objectKey: string;
  expiresInMinutes: number;
  maxDownloads?: number;
  downloadFilename: string;
}

const initialLinkFormState: LinkFormState = {
  bucket: "",
  objectKey: "",
  expiresInMinutes: 60,
  maxDownloads: undefined,
  downloadFilename: "",
};

const App = () => {
  const toast = useToast();
  const { isOpen, onOpen, onClose } = useDisclosure();
  const cancelRef = React.useRef<HTMLButtonElement>(null);
  const [authToken, setAuthToken] = useState<string | null>(null);
  const [username, setUsername] = useState<string | null>(null);
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [linkForm, setLinkForm] = useState<LinkFormState>(initialLinkFormState);
  const [enforceLimit, setEnforceLimit] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [links, setLinks] = useState<CreateLinkResponse[]>([]);
  const [historyLinks, setHistoryLinks] = useState<DownloadLinkResponse[]>([]);
  const [linkToDelete, setLinkToDelete] = useState<DownloadLinkResponse | null>(
    null
  );
  const [buckets, setBuckets] = useState<Bucket[]>([]);
  const [isLoadingBuckets, setIsLoadingBuckets] = useState(false);
  const [objects, setObjects] = useState<ObjectInfo[]>([]);
  const [isLoadingObjects, setIsLoadingObjects] = useState(false);

  useEffect(() => {
    const storedToken = localStorage.getItem(TOKEN_STORAGE_KEY);
    const storedExpiry = localStorage.getItem(TOKEN_EXPIRY_STORAGE_KEY);
    const storedUsername = localStorage.getItem(USERNAME_STORAGE_KEY);
    if (storedToken && storedExpiry) {
      const expiresAt = Number(storedExpiry);
      if (Number.isFinite(expiresAt) && expiresAt > Date.now()) {
        setAuthToken(storedToken);
        setUsername(storedUsername);
      } else {
        localStorage.removeItem(TOKEN_STORAGE_KEY);
        localStorage.removeItem(TOKEN_EXPIRY_STORAGE_KEY);
        localStorage.removeItem(USERNAME_STORAGE_KEY);
      }
    }

    // 处理 OAuth2 回调
    const handleOAuthCallback = async () => {
      const params = new URLSearchParams(window.location.search);
      const code = params.get("code");
      const state = params.get("state");

      if (code && state) {
        // 清除 URL 参数
        window.history.replaceState(
          {},
          document.title,
          window.location.pathname
        );

        // 获取存储的 OAuth session
        const session = getAndClearOAuthSession();
        if (!session || session.state !== state) {
          toast({
            title: "登录失败",
            description: "OAuth2 状态验证失败，请重试。",
            status: "error",
            duration: 3000,
            isClosable: true,
          });
          return;
        }

        setIsLoggingIn(true);
        try {
          const response = await axios.get<LoginResponse>(
            `${API_CONFIG.BASE_URL}/api/oauth/callback`,
            {
              params: {
                code,
                state,
                code_verifier: session.codeVerifier,
              },
            }
          );

          const {
            token,
            expires_in,
            username: responseUsername,
          } = response.data;
          setAuthToken(token);
          setUsername(responseUsername || null);

          const expiresAt = Date.now() + expires_in * 1000;
          localStorage.setItem(TOKEN_STORAGE_KEY, token);
          localStorage.setItem(TOKEN_EXPIRY_STORAGE_KEY, String(expiresAt));
          if (responseUsername) {
            localStorage.setItem(USERNAME_STORAGE_KEY, responseUsername);
          }

          toast({
            title: "登录成功",
            description: `欢迎回来，${responseUsername || "用户"}！`,
            status: "success",
            duration: 2500,
            isClosable: true,
          });
        } catch (error) {
          console.error("OAuth callback error:", error);
          toast({
            title: "登录失败",
            description: "OAuth2 认证失败，请重试。",
            status: "error",
            duration: 3000,
            isClosable: true,
          });
        } finally {
          setIsLoggingIn(false);
        }
      }
    };

    handleOAuthCallback();
  }, [toast]);

  const axiosInstance = useMemo(() => {
    const instance = axios.create({
      baseURL: API_CONFIG.BASE_URL,
    });

    instance.interceptors.request.use((config) => {
      if (authToken) {
        config.headers.Authorization = `Bearer ${authToken}`;
      }
      return config;
    });

    return instance;
  }, [authToken]);

  const fetchHistoryLinks = useCallback(async () => {
    if (!authToken) return;
    try {
      const response = await axiosInstance.get<ListLinksResponse>("/links");
      setHistoryLinks(response.data?.links || []);
    } catch (error) {
      console.error("Failed to fetch history links:", error);
      setHistoryLinks([]);
    }
  }, [axiosInstance, authToken]);

  const fetchBuckets = useCallback(async () => {
    if (!authToken) return;
    setIsLoadingBuckets(true);
    try {
      const response = await axiosInstance.get<ListBucketsResponse>("/buckets");
      setBuckets(response.data?.buckets || []);
    } catch (error) {
      console.error("Failed to fetch buckets:", error);
      setBuckets([]);
      toast({
        title: "获取 Bucket 列表失败",
        description: "请检查网络连接或联系管理员",
        status: "error",
        duration: 5000,
        isClosable: true,
      });
    } finally {
      setIsLoadingBuckets(false);
    }
  }, [axiosInstance, authToken, toast]);

  const fetchObjects = useCallback(
    async (bucketName: string) => {
      if (!authToken || !bucketName) return;
      setIsLoadingObjects(true);
      try {
        const aggregatedObjects: ObjectInfo[] = [];
        const MAX_PAGES = 100; // 避免意外的无限循环
        let continuationToken: string | undefined;

        for (let page = 0; page < MAX_PAGES; page += 1) {
          const params: Record<string, string> = { bucket: bucketName };
          if (continuationToken) {
            params["continuation-token"] = continuationToken;
          }

          const { data } = await axiosInstance.get<ListObjectsResponse>(
            "/objects",
            { params }
          );

          if (!data) {
            break;
          }

          aggregatedObjects.push(...data.objects);

          if (data.is_truncated && data.next_continuation_token) {
            continuationToken = data.next_continuation_token;
          } else {
            continuationToken = undefined;
            break;
          }
        }

        setObjects(aggregatedObjects);
      } catch (error) {
        console.error("Failed to fetch objects:", error);
        setObjects([]);
        toast({
          title: "获取对象列表失败",
          description: "请检查网络连接或联系管理员",
          status: "error",
          duration: 5000,
          isClosable: true,
        });
      } finally {
        setIsLoadingObjects(false);
      }
    },
    [axiosInstance, authToken, toast]
  );

  useEffect(() => {
    if (authToken) {
      fetchHistoryLinks();
      fetchBuckets();
    }
  }, [authToken, fetchHistoryLinks, fetchBuckets]);

  // 当bucket选择变化时，获取对应的objects
  useEffect(() => {
    if (linkForm.bucket) {
      fetchObjects(linkForm.bucket);
    } else {
      setObjects([]);
    }
  }, [linkForm.bucket, fetchObjects]);

  const handleLogin = useCallback(async () => {
    // 生成 PKCE 参数
    const codeVerifier = generateCodeVerifier();
    const codeChallenge = await generateCodeChallenge(codeVerifier);
    const state = generateState();

    // 存储到 sessionStorage
    storeOAuthSession(state, codeVerifier);

    // 构建授权 URL 并重定向
    const authorizeUrl = buildAuthorizeUrl(
      OAUTH_CONFIG.AUTHORIZE_URL,
      OAUTH_CONFIG.CLIENT_ID,
      OAUTH_CONFIG.REDIRECT_URI,
      state,
      codeChallenge,
      OAUTH_CONFIG.SCOPE
    );

    window.location.href = authorizeUrl;
  }, []);

  const handleLogout = useCallback(() => {
    setAuthToken(null);
    setUsername(null);
    localStorage.removeItem(TOKEN_STORAGE_KEY);
    localStorage.removeItem(TOKEN_EXPIRY_STORAGE_KEY);
    localStorage.removeItem(USERNAME_STORAGE_KEY);
    toast({
      title: "已退出登录",
      status: "info",
      duration: 2000,
      isClosable: true,
    });
  }, [toast]);

  const handleCreateLink = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (!authToken) {
        toast({
          title: "需要登录",
          description: "请先登录后再生成下载链接。",
          status: "warning",
        });
        return;
      }

      if (!linkForm.objectKey.trim()) {
        toast({
          title: "对象键不能为空",
          status: "warning",
        });
        return;
      }

      setIsSubmitting(true);
      try {
        const payload: CreateLinkRequest = {
          object_key: linkForm.objectKey.trim(),
          expires_in_seconds: Math.round(linkForm.expiresInMinutes * 60),
        };
        if (linkForm.bucket.trim()) {
          payload.bucket = linkForm.bucket.trim();
          // 从选择的bucket中获取endpoint
          const selectedBucket = buckets.find(
            (b) => b.name === linkForm.bucket.trim()
          );
          if (selectedBucket) {
            payload.endpoint = selectedBucket.extranet_endpoint;
          }
        }
        if (linkForm.downloadFilename.trim()) {
          payload.download_filename = linkForm.downloadFilename.trim();
        }
        if (enforceLimit) {
          if (!linkForm.maxDownloads || linkForm.maxDownloads <= 0) {
            toast({
              title: "最大下载次数必须大于 0",
              status: "warning",
            });
            setIsSubmitting(false);
            return;
          }
          payload.max_downloads = linkForm.maxDownloads;
        }

        const response = await axiosInstance.post<CreateLinkResponse>(
          "/sign",
          payload
        );
        setLinks((prev) => [response.data, ...prev]);
        // 刷新历史链接列表
        fetchHistoryLinks();
        toast({
          title: "下载链接生成成功",
          status: "success",
          duration: 2500,
          isClosable: true,
        });
      } catch (error) {
        console.error(error);
        toast({
          title: "生成失败",
          description: "请检查填写信息，稍后重试。",
          status: "error",
          duration: 3000,
          isClosable: true,
        });
      } finally {
        setIsSubmitting(false);
      }
    },
    [authToken, enforceLimit, linkForm, axiosInstance, toast]
  );

  const resetLinkForm = useCallback(() => {
    setLinkForm(initialLinkFormState);
    setEnforceLimit(false);
  }, []);

  const copyToClipboard = useCallback(
    async (value: string) => {
      try {
        await navigator.clipboard.writeText(value);
        toast({ title: "已复制到剪贴板", status: "success", duration: 1500 });
      } catch (error) {
        console.error(error);
        toast({ title: "复制失败", status: "error", duration: 2000 });
      }
    },
    [toast]
  );

  const handleDeleteClick = useCallback(
    (link: DownloadLinkResponse) => {
      setLinkToDelete(link);
      onOpen();
    },
    [onOpen]
  );

  const confirmDelete = useCallback(async () => {
    if (!linkToDelete) return;

    try {
      await axiosInstance.delete(`/links/${linkToDelete.id}`);
      toast({
        title: "链接已删除",
        status: "success",
        duration: 2000,
        isClosable: true,
      });
      // 刷新历史链接列表
      fetchHistoryLinks();
    } catch (error) {
      console.error(error);
      toast({
        title: "删除失败",
        description: "请稍后重试",
        status: "error",
        duration: 3000,
        isClosable: true,
      });
    } finally {
      onClose();
      setLinkToDelete(null);
    }
  }, [axiosInstance, toast, fetchHistoryLinks, linkToDelete, onClose]);

  const renderLoginForm = () => (
    <Box bg="white" boxShadow="md" borderRadius="lg" p={8} w="100%">
      <Heading size="md" mb={6} textAlign="center">
        管理员登录
      </Heading>
      <VStack spacing={4} align="stretch">
        <Text textAlign="center" color="gray.600">
          使用统一身份认证系统登录
        </Text>
        <Button
          colorScheme="blue"
          onClick={handleLogin}
          isLoading={isLoggingIn}
          size="lg"
        >
          使用 SSO 登录
        </Button>
      </VStack>
    </Box>
  );

  const renderLinkForm = () => (
    <Box
      as="form"
      onSubmit={handleCreateLink}
      bg="white"
      boxShadow="md"
      borderRadius="lg"
      p={8}
    >
      <Flex align="center" mb={6}>
        <Heading size="md">生成下载链接</Heading>
        <Spacer />
        <HStack spacing={3}>
          {username && (
            <Text color="gray.600" fontSize="sm">
              Hi {username}!
            </Text>
          )}
          <Button size="sm" variant="outline" onClick={handleLogout}>
            退出登录
          </Button>
        </HStack>
      </Flex>
      <VStack spacing={5} align="stretch">
        <FormControl>
          <FormLabel>Bucket</FormLabel>
          <HStack>
            <Select
              value={linkForm.bucket}
              onChange={(e) =>
                setLinkForm((prev) => ({ ...prev, bucket: e.target.value }))
              }
              placeholder="选择 Bucket（可选，留空使用默认）"
              isDisabled={isLoadingBuckets}
            >
              {buckets?.map((bucket) => (
                <option key={bucket.name} value={bucket.name}>
                  {bucket.name} ({bucket.location})
                </option>
              ))}
            </Select>
            <IconButton
              aria-label="刷新 Bucket 列表"
              icon={<RepeatIcon />}
              onClick={fetchBuckets}
              isLoading={isLoadingBuckets}
              size="md"
              variant="outline"
            />
          </HStack>
        </FormControl>
        <FormControl isRequired>
          <FormLabel>对象键（Object Key）</FormLabel>
          <HStack>
            <Select
              value={linkForm.objectKey}
              onChange={(e) =>
                setLinkForm((prev) => ({ ...prev, objectKey: e.target.value }))
              }
              placeholder={linkForm.bucket ? "选择对象..." : "请先选择 Bucket"}
              isDisabled={!linkForm.bucket || isLoadingObjects}
            >
              {objects?.map((obj) => (
                <option key={obj.key} value={obj.key}>
                  {obj.key} ({(obj.size / 1024 / 1024).toFixed(2)} MB)
                </option>
              ))}
            </Select>
            <IconButton
              aria-label="刷新对象列表"
              icon={<RepeatIcon />}
              onClick={() => linkForm.bucket && fetchObjects(linkForm.bucket)}
              isLoading={isLoadingObjects}
              isDisabled={!linkForm.bucket}
              size="md"
              variant="outline"
            />
          </HStack>
        </FormControl>
        <FormControl>
          <FormLabel>下载文件名（可选）</FormLabel>
          <Input
            value={linkForm.downloadFilename}
            onChange={(e) =>
              setLinkForm((prev) => ({
                ...prev,
                downloadFilename: e.target.value,
              }))
            }
            placeholder="下载时显示的文件名"
          />
        </FormControl>
        <FormControl>
          <FormLabel>有效期（分钟）</FormLabel>
          <NumberInput
            min={1}
            value={linkForm.expiresInMinutes}
            onChange={(_, value) =>
              setLinkForm((prev) => ({
                ...prev,
                expiresInMinutes: Number.isNaN(value) ? 60 : value,
              }))
            }
          >
            <NumberInputField />
          </NumberInput>
        </FormControl>
        <FormControl display="flex" alignItems="center">
          <HStack spacing={3}>
            <Switch
              isChecked={enforceLimit}
              onChange={(e) => setEnforceLimit(e.target.checked)}
            />
            <FormLabel m={0}>限制下载次数</FormLabel>
            <NumberInput
              min={1}
              value={linkForm.maxDownloads ?? ""}
              onChange={(_, value) =>
                setLinkForm((prev) => ({
                  ...prev,
                  maxDownloads: Number.isNaN(value) ? undefined : value,
                }))
              }
              isDisabled={!enforceLimit}
              w="120px"
            >
              <NumberInputField placeholder="默认10次" />
            </NumberInput>
          </HStack>
        </FormControl>
        <HStack spacing={4}>
          <Button colorScheme="blue" type="submit" isLoading={isSubmitting}>
            生成链接
          </Button>
          <Button
            leftIcon={<RepeatIcon />}
            variant="ghost"
            onClick={resetLinkForm}
          >
            重置
          </Button>
        </HStack>
      </VStack>
    </Box>
  );

  const renderLinkList = () => (
    <Box bg="white" boxShadow="md" borderRadius="lg" p={6}>
      <HStack spacing={4} mb={4} align="center">
        <Heading size="md">历史链接</Heading>
        <IconButton
          aria-label="刷新列表"
          icon={<RepeatIcon />}
          onClick={fetchHistoryLinks}
          variant="ghost"
          size="sm"
        />
      </HStack>
      {(historyLinks?.length || 0) === 0 ? (
        <Text color="gray.500">还没有生成链接。</Text>
      ) : (
        <Stack spacing={4} divider={<Divider />}>
          {historyLinks?.map((link) => (
            <Box key={link.id}>
              <HStack spacing={3} align="flex-start">
                <Box flex="1">
                  <Text
                    fontWeight="semibold"
                    color={link.is_expired ? "gray.400" : "black"}
                  >
                    {link.object_key}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    {link.download_filename &&
                      `文件名：${link.download_filename}`}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    过期时间：{new Date(link.expires_at).toLocaleString()}
                  </Text>
                  <Text fontSize="sm" color="gray.600">
                    创建时间：{new Date(link.created_at).toLocaleString()}
                  </Text>
                  <HStack spacing={2} mt={2}>
                    {/* 状态标签 */}
                    {link.is_expired ? (
                      <Badge colorScheme="red">已过期</Badge>
                    ) : (
                      <Badge colorScheme="green">有效</Badge>
                    )}

                    {/* 使用次数标签 */}
                    {link.max_downloads ? (
                      <Badge
                        colorScheme={
                          link.downloads_served >= link.max_downloads
                            ? "red"
                            : "blue"
                        }
                      >
                        已使用 {link.downloads_served}/{link.max_downloads} 次
                      </Badge>
                    ) : (
                      <Badge colorScheme="purple">
                        已使用 {link.downloads_served} 次 (无限制)
                      </Badge>
                    )}
                  </HStack>
                </Box>
                <IconButton
                  aria-label="复制链接"
                  icon={<CopyIcon />}
                  onClick={() => copyToClipboard(link.download_url)}
                  variant="ghost"
                  isDisabled={
                    link.is_expired ||
                    (!!link.max_downloads &&
                      link.downloads_served >= link.max_downloads)
                  }
                />
                <Link
                  href={link.download_url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <Button
                    variant="outline"
                    size="sm"
                    isDisabled={
                      link.is_expired ||
                      (!!link.max_downloads &&
                        link.downloads_served >= link.max_downloads)
                    }
                  >
                    下载
                  </Button>
                </Link>
                <IconButton
                  aria-label="删除链接"
                  icon={<DeleteIcon />}
                  onClick={() => handleDeleteClick(link)}
                  variant="ghost"
                  colorScheme="red"
                  size="sm"
                />
              </HStack>
            </Box>
          ))}
        </Stack>
      )}
    </Box>
  );

  return (
    <Box bg="gray.50" minH="100vh" py={10}>
      <Container maxW="4xl">
        <VStack spacing={8} align="stretch">
          <Heading textAlign="center">OSS Signed URL Generator</Heading>
          {!authToken ? (
            renderLoginForm()
          ) : (
            <>
              {renderLinkForm()}
              {renderLinkList()}
            </>
          )}
          <Box textAlign="center" color="gray.500" fontSize="sm">
            <VStack spacing={2}>
              <Text>
                <Link
                  href="https://beian.miit.gov.cn/"
                  target="_blank"
                  color="blue.500"
                >
                  鲁ICP备2024119517号-1
                </Link>
              </Text>
              <Text>
                Copyright © 2025
                {new Date().getFullYear() > 2025
                  ? `-${new Date().getFullYear()}`
                  : ""}{" "}
                Honahec
              </Text>
            </VStack>
          </Box>
        </VStack>
      </Container>

      {/* 确认删除对话框 */}
      <AlertDialog
        isOpen={isOpen}
        leastDestructiveRef={cancelRef}
        onClose={onClose}
      >
        <AlertDialogOverlay>
          <AlertDialogContent>
            <AlertDialogHeader fontSize="lg" fontWeight="bold">
              删除链接
            </AlertDialogHeader>
            <AlertDialogBody>
              确定要删除此下载链接吗？
              {linkToDelete && (
                <Box mt={2} p={2} bg="gray.50" borderRadius="md">
                  <Text fontSize="sm" fontWeight="semibold">
                    {linkToDelete.object_key}
                  </Text>
                  {linkToDelete.download_filename && (
                    <Text fontSize="xs" color="gray.600">
                      文件名：{linkToDelete.download_filename}
                    </Text>
                  )}
                </Box>
              )}
              <Text mt={2} color="red.500">
                此操作不可撤销。
              </Text>
            </AlertDialogBody>
            <AlertDialogFooter>
              <Button ref={cancelRef} onClick={onClose}>
                取消
              </Button>
              <Button colorScheme="red" onClick={confirmDelete} ml={3}>
                删除
              </Button>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialogOverlay>
      </AlertDialog>
    </Box>
  );
};

export default App;
