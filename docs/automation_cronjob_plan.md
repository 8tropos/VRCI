# Oracle Automation & Cronjob System

## Overview
Comprehensive off-chain automation system for fetching price data from multiple APIs and updating the oracle contract with validated, consensus-based pricing information.

## Architecture

```
APIs (CoinGecko, CoinMarketCap, Binance) 
    ↓
Data Aggregation Service
    ↓
Validation & Consensus Engine
    ↓
Transaction Queue Manager
    ↓
Oracle Contract Updates
```

## Implementation Phases

### **Phase 1: Basic Price Fetcher (Week 1)**

#### **1.1 Core API Integration**

**Technologies:** Node.js, TypeScript, dotenv, node-cron

**Structure:**
```
oracle-updater/
├── src/
│   ├── apis/
│   │   ├── coingecko.ts
│   │   ├── coinmarketcap.ts
│   │   ├── binance.ts
│   │   └── base-api.ts
│   ├── config/
│   │   ├── tokens.ts
│   │   ├── api-keys.ts
│   │   └── oracle-config.ts
│   ├── services/
│   │   ├── price-fetcher.ts
│   │   ├── validator.ts
│   │   └── contract-client.ts
│   └── main.ts
├── config/
│   ├── production.env
│   ├── testnet.env
│   └── tokens.json
└── package.json
```

**Key Components:**

```typescript
// Base API interface
interface PriceAPI {
    name: string;
    fetchPrice(symbol: string): Promise<PriceData>;
    fetchBatch(symbols: string[]): Promise<Map<string, PriceData>>;
    isHealthy(): Promise<boolean>;
}

// Price data structure
interface PriceData {
    price: number;
    marketCap: number;
    volume24h: number;
    timestamp: number;
    source: string;
    confidence: number; // 0-100
}

// Token configuration
interface TokenConfig {
    address: string;
    symbol: string;
    apis: string[];          // Which APIs to use
    updateFrequency: number; // Seconds between updates
    deviation: number;       // Max allowed price deviation %
    minSources: number;      // Minimum sources required
}
```

#### **1.2 Simple Scheduler**

```typescript
// Basic cron implementation
class OracleUpdater {
    private tokens: TokenConfig[];
    private apis: Map<string, PriceAPI>;
    
    constructor() {
        this.loadConfiguration();
        this.initializeAPIs();
    }
    
    // Main update loop
    async updateAllPrices(): Promise<void> {
        const updates = [];
        
        for (const token of this.tokens) {
            try {
                const priceData = await this.fetchTokenPrice(token);
                if (this.validatePrice(token, priceData)) {
                    updates.push({ token, priceData });
                }
            } catch (error) {
                console.error(`Failed to update ${token.symbol}:`, error);
            }
        }
        
        if (updates.length > 0) {
            await this.submitBatchUpdate(updates);
        }
    }
    
    // Schedule regular updates
    start(): void {
        // Run every 5 minutes
        cron.schedule('*/5 * * * *', () => {
            this.updateAllPrices();
        });
    }
}
```

### **Phase 2: Multi-Source Validation (Week 2)**

#### **2.1 Data Aggregation Engine**

```typescript
class PriceAggregator {
    // Fetch from multiple sources
    async fetchMultiSourcePrice(token: TokenConfig): Promise<AggregatedPrice> {
        const promises = token.apis.map(apiName => 
            this.apis.get(apiName).fetchPrice(token.symbol)
                .catch(error => ({ error, source: apiName }))
        );
        
        const results = await Promise.allSettled(promises);
        const validPrices = results
            .filter(result => result.status === 'fulfilled' && !result.value.error)
            .map(result => result.value as PriceData);
            
        return this.calculateConsensus(validPrices, token.minSources);
    }
    
    // Calculate consensus price using median
    private calculateConsensus(prices: PriceData[], minSources: number): AggregatedPrice {
        if (prices.length < minSources) {
            throw new Error(`Insufficient sources: ${prices.length} < ${minSources}`);
        }
        
        const sortedPrices = prices.map(p => p.price).sort((a, b) => a - b);
        const medianPrice = this.calculateMedian(sortedPrices);
        
        // Check for outliers
        const maxDeviation = this.calculateMaxDeviation(sortedPrices, medianPrice);
        if (maxDeviation > 0.05) { // 5% max deviation
            throw new Error(`Price deviation too high: ${maxDeviation * 100}%`);
        }
        
        return {
            price: medianPrice,
            marketCap: this.calculateMedian(prices.map(p => p.marketCap)),
            volume24h: this.calculateMedian(prices.map(p => p.volume24h)),
            sourceCount: prices.length,
            confidence: this.calculateConfidence(prices),
            timestamp: Date.now()
        };
    }
}
```

#### **2.2 Validation Rules Engine**

```typescript
class PriceValidator {
    // Validate price against historical data
    validatePriceChange(token: TokenConfig, newPrice: number, lastPrice: number): boolean {
        if (!lastPrice) return true; // First price
        
        const change = Math.abs(newPrice - lastPrice) / lastPrice;
        return change <= (token.deviation / 100);
    }
    
    // Validate data freshness
    validateTimestamp(timestamp: number, maxAge: number = 300): boolean {
        const age = (Date.now() - timestamp) / 1000; // seconds
        return age <= maxAge;
    }
    
    // Validate price range
    validatePriceRange(price: number): boolean {
        return price > 0 && price < Number.MAX_SAFE_INTEGER;
    }
    
    // Comprehensive validation
    validate(token: TokenConfig, aggregatedPrice: AggregatedPrice, lastPrice?: number): ValidationResult {
        const checks = [
            this.validatePriceRange(aggregatedPrice.price),
            this.validateTimestamp(aggregatedPrice.timestamp),
            this.validatePriceChange(token, aggregatedPrice.price, lastPrice),
            aggregatedPrice.sourceCount >= token.minSources
        ];
        
        return {
            isValid: checks.every(check => check),
            errors: this.collectValidationErrors(checks),
            warnings: this.collectValidationWarnings(aggregatedPrice)
        };
    }
}
```

### **Phase 3: Blockchain Integration (Week 3)**

#### **3.1 Contract Client**

```typescript
class OracleContractClient {
    private api: ApiPromise;
    private keyring: KeyringPair;
    private contractAddress: string;
    
    constructor(rpcUrl: string, privateKey: string, contractAddress: string) {
        this.initializeApi(rpcUrl);
        this.keyring = new Keyring({ type: 'sr25519' }).addFromUri(privateKey);
        this.contractAddress = contractAddress;
    }
    
    // Single token update
    async updateTokenPrice(token: string, priceData: AggregatedPrice): Promise<string> {
        const tx = this.api.tx.contracts.call(
            this.contractAddress,
            0, // value
            GAS_LIMIT,
            'update_token_data',
            [token, priceData.price, priceData.marketCap, priceData.volume24h]
        );
        
        return new Promise((resolve, reject) => {
            tx.signAndSend(this.keyring, ({ status, events }) => {
                if (status.isInBlock) {
                    const success = events.some(record => 
                        this.api.events.contracts.ContractEmitted.is(record.event)
                    );
                    
                    if (success) {
                        resolve(status.asInBlock.toString());
                    } else {
                        reject(new Error('Transaction failed'));
                    }
                }
            });
        });
    }
    
    // Batch update for efficiency
    async batchUpdatePrices(updates: TokenUpdate[]): Promise<string> {
        if (updates.length === 0) return;
        
        const batchData = updates.map(update => [
            update.token,
            update.priceData.price,
            update.priceData.marketCap,
            update.priceData.volume24h
        ]);
        
        return this.callContract('batch_update_prices', [batchData]);
    }
    
    // Get current price for validation
    async getCurrentPrice(token: string): Promise<number | null> {
        const result = await this.queryContract('get_price', [token]);
        return result.isOk ? result.value : null;
    }
}
```

#### **3.2 Transaction Queue Manager**

```typescript
class TransactionQueue {
    private queue: TransactionItem[] = [];
    private processing = false;
    private gasTracker = new GasTracker();
    
    // Add transaction to queue
    enqueue(item: TransactionItem): void {
        this.queue.push(item);
        if (!this.processing) {
            this.processQueue();
        }
    }
    
    // Process queue with rate limiting
    private async processQueue(): Promise<void> {
        this.processing = true;
        
        while (this.queue.length > 0) {
            const item = this.queue.shift();
            
            try {
                // Check gas costs
                await this.gasTracker.ensureSufficientGas();
                
                // Execute transaction
                const txHash = await item.execute();
                
                // Track success
                this.logSuccess(item, txHash);
                
                // Rate limiting (prevent spam)
                await this.delay(2000); // 2 seconds between transactions
                
            } catch (error) {
                // Handle failures with exponential backoff
                await this.handleFailure(item, error);
            }
        }
        
        this.processing = false;
    }
    
    // Retry failed transactions
    private async handleFailure(item: TransactionItem, error: Error): Promise<void> {
        if (item.retryCount < MAX_RETRIES) {
            item.retryCount++;
            
            // Exponential backoff: 2^retryCount seconds
            const delay = Math.pow(2, item.retryCount) * 1000;
            setTimeout(() => this.enqueue(item), delay);
            
            console.warn(`Transaction failed, retrying in ${delay}ms:`, error);
        } else {
            console.error(`Transaction failed permanently after ${MAX_RETRIES} retries:`, error);
            this.alertFailure(item, error);
        }
    }
}
```

### **Phase 4: Monitoring & Reliability (Week 4)**

#### **4.1 Health Monitoring**

```typescript
class HealthMonitor {
    private metrics = {
        apiSuccessRates: new Map<string, number>(),
        updateLatency: [],
        transactionSuccessRate: 0,
        lastSuccessfulUpdate: 0,
        gasUsage: []
    };
    
    // Monitor API health
    async checkAPIHealth(): Promise<HealthReport> {
        const results = await Promise.allSettled(
            Array.from(this.apis.values()).map(api => api.isHealthy())
        );
        
        const healthyAPIs = results.filter(result => 
            result.status === 'fulfilled' && result.value
        ).length;
        
        return {
            healthyAPIs,
            totalAPIs: this.apis.size,
            isHealthy: healthyAPIs >= MIN_HEALTHY_APIS,
            timestamp: Date.now()
        };
    }
    
    // Monitor update performance
    trackUpdatePerformance(duration: number, success: boolean): void {
        this.metrics.updateLatency.push(duration);
        
        // Keep only last 100 measurements
        if (this.metrics.updateLatency.length > 100) {
            this.metrics.updateLatency.shift();
        }
        
        if (success) {
            this.metrics.lastSuccessfulUpdate = Date.now();
        }
    }
    
    // Generate alerts
    checkAlertConditions(): Alert[] {
        const alerts = [];
        
        // Stale data alert
        const timeSinceUpdate = Date.now() - this.metrics.lastSuccessfulUpdate;
        if (timeSinceUpdate > STALE_THRESHOLD) {
            alerts.push({
                level: 'critical',
                message: `No successful updates for ${timeSinceUpdate / 1000}s`,
                timestamp: Date.now()
            });
        }
        
        // API failure alert
        const healthReport = await this.checkAPIHealth();
        if (!healthReport.isHealthy) {
            alerts.push({
                level: 'warning',
                message: `Only ${healthReport.healthyAPIs}/${healthReport.totalAPIs} APIs healthy`,
                timestamp: Date.now()
            });
        }
        
        return alerts;
    }
}
```

#### **4.2 Alerting System**

```typescript
class AlertManager {
    private alertChannels: AlertChannel[] = [];
    
    // Add notification channels
    addChannel(channel: AlertChannel): void {
        this.alertChannels.push(channel);
    }
    
    // Send alert to all channels
    async sendAlert(alert: Alert): Promise<void> {
        const promises = this.alertChannels.map(channel => 
            channel.send(alert).catch(error => 
                console.error(`Failed to send alert via ${channel.name}:`, error)
            )
        );
        
        await Promise.allSettled(promises);
    }
}

// Alert channel implementations
class EmailAlertChannel implements AlertChannel {
    async send(alert: Alert): Promise<void> {
        // Send email notification
    }
}

class SlackAlertChannel implements AlertChannel {
    async send(alert: Alert): Promise<void> {
        // Send Slack notification
    }
}

class DiscordAlertChannel implements AlertChannel {
    async send(alert: Alert): Promise<void> {
        // Send Discord notification
    }
}
```

### **Phase 5: Advanced Features (Week 5-6)**

#### **5.1 Dynamic Configuration**

```typescript
class ConfigManager {
    private config: DynamicConfig;
    private configFile: string;
    
    // Hot reload configuration
    watchConfigChanges(): void {
        fs.watchFile(this.configFile, (curr, prev) => {
            if (curr.mtime !== prev.mtime) {
                this.reloadConfig();
            }
        });
    }
    
    // Adjust update frequency based on volatility
    adjustUpdateFrequency(token: string, volatility: number): void {
        const config = this.config.tokens.get(token);
        
        if (volatility > HIGH_VOLATILITY_THRESHOLD) {
            config.updateFrequency = Math.min(config.updateFrequency, 60); // 1 minute
        } else if (volatility < LOW_VOLATILITY_THRESHOLD) {
            config.updateFrequency = Math.max(config.updateFrequency, 600); // 10 minutes
        }
        
        this.saveConfig();
    }
    
    // Emergency mode activation
    activateEmergencyMode(reason: string): void {
        this.config.emergencyMode = true;
        this.config.emergencyFrequency = 30; // 30 seconds
        
        // Alert all systems
        this.alertManager.sendAlert({
            level: 'critical',
            message: `Emergency mode activated: ${reason}`,
            timestamp: Date.now()
        });
    }
}
```

#### **5.2 Performance Optimization**

```typescript
class PerformanceOptimizer {
    // Batch similar API calls
    optimizeBatchCalls(tokens: TokenConfig[]): Map<string, string[]> {
        const batches = new Map<string, string[]>();
        
        for (const token of tokens) {
            for (const apiName of token.apis) {
                if (!batches.has(apiName)) {
                    batches.set(apiName, []);
                }
                batches.get(apiName).push(token.symbol);
            }
        }
        
        return batches;
    }
    
    // Parallel processing with concurrency limits
    async processWithConcurrency<T>(
        items: T[], 
        processor: (item: T) => Promise<any>, 
        concurrency: number = 5
    ): Promise<any[]> {
        const results = [];
        
        for (let i = 0; i < items.length; i += concurrency) {
            const batch = items.slice(i, i + concurrency);
            const batchResults = await Promise.allSettled(
                batch.map(processor)
            );
            results.push(...batchResults);
        }
        
        return results;
    }
}
```

## Deployment & Operations

### **Environment Setup**

```bash
# Production environment
NODE_ENV=production
RPC_URL=wss://rpc.polkadot.io
ORACLE_CONTRACT=5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
PRIVATE_KEY=//Alice  # Use secure key management in production

# API Keys
COINGECKO_API_KEY=your_key_here
COINMARKETCAP_API_KEY=your_key_here
BINANCE_API_KEY=your_key_here

# Monitoring
ALERT_EMAIL=alerts@yourproject.com
SLACK_WEBHOOK=https://hooks.slack.com/...
DISCORD_WEBHOOK=https://discord.com/api/webhooks/...
```

### **Docker Deployment**

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY src/ ./src/
COPY config/ ./config/

CMD ["npm", "start"]
```

### **Monitoring Dashboard**

- **Grafana**: Visualize update frequency, API success rates, gas usage
- **Prometheus**: Collect metrics from the automation system
- **AlertManager**: Handle alert routing and deduplication

### **Backup & Recovery**

- **Configuration Backup**: Daily snapshots of token configurations
- **State Recovery**: Ability to resume from last successful update
- **Failover**: Automatic switching to backup APIs when primary fails

## Success Metrics

### **Reliability**
- **99.5% Update Success Rate**: Less than 0.5% failed price updates
- **<5 Minute Recovery**: Quick recovery from API failures
- **<1 Hour Alert Response**: Fast response to critical issues

### **Performance**
- **<30 Second Update Latency**: From API fetch to blockchain update
- **<$10/day Operating Costs**: Efficient gas usage and API costs
- **Multi-Source Consensus**: Always use 2+ sources for price validation

### **Data Quality**
- **<2% Price Deviation**: Consensus mechanism catches outliers
- **100% Data Freshness**: No stale prices in registry system
- **Zero Manual Intervention**: Fully automated operation
